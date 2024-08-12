use plonky2::plonk::proof::ProofWithPublicInputs;
use rayon::prelude::*;

use crate::{
    account::Account,
    circuit_registry::registry::CircuitRegistry,
    merkle_sum_prover::prover::MerkleSumTreeProver,
    recursive_prover::prover::RecursiveProver,
    types::{C, D, F},
};

pub fn batch_prove_accounts<const RECURSION_BRANCHOUT_NUM: usize>(
    circuit_registry: &CircuitRegistry<RECURSION_BRANCHOUT_NUM>,
    account_batches: Vec<Account>,
    parallism: usize,
    batch_size: usize,
) -> Vec<ProofWithPublicInputs<F, C, D>> {
    let mut batch_proofs: Vec<ProofWithPublicInputs<F, C, D>> = Vec::new();
    let (batch_circuit, account_targets) = circuit_registry.get_batch_circuit();

    let _ = account_batches
        .chunks(parallism * batch_size)
        .map(|chunk| {
            let proofs: Vec<ProofWithPublicInputs<F, C, D>> = chunk
                .par_chunks(batch_size)
                .map(|accounts| {
                    let prover = MerkleSumTreeProver { accounts: accounts.to_owned() };
                    let proof =
                        prover.get_proof_with_circuit_data(&account_targets, &batch_circuit);
                    proof
                    // TODO: parse tree node from proof and check against the one generated by merkle sum tree.
                })
                .collect();
            batch_proofs.extend(proofs.into_iter());
        })
        .collect::<Vec<_>>();

    batch_proofs
}

pub fn recursive_prove_subproofs<const RECURSION_BRANCHOUT_NUM: usize>(
    subproofs: Vec<ProofWithPublicInputs<F, C, D>>,
    circuit_registry: &CircuitRegistry<RECURSION_BRANCHOUT_NUM>,
    parallism: usize,
) -> ProofWithPublicInputs<F, C, D> {
    let (batch_circuit, _) = circuit_registry.get_batch_circuit();
    let mut last_level_circuit_vd = batch_circuit.verifier_only.clone();
    let mut last_level_proofs = subproofs;
    let recursive_levels = circuit_registry.get_recursive_levels();
    tracing::info!("total recursive levels: {:?}", recursive_levels);

    for level in 0..recursive_levels {
        let start = std::time::Instant::now();
        let last_level_vd_digest = last_level_circuit_vd.circuit_digest;
        let last_level_empty_proof =
            circuit_registry.get_empty_proof(&last_level_vd_digest).expect(
                format!(
                    "fail to find empty proof at recursive level {} with inner circuit vd {:?}",
                    level, last_level_vd_digest
                )
                .as_str(),
            );

        let subproof_len = last_level_proofs.len();
        tracing::info!("Start on recursive Level {}, number of subproofs {}", level, subproof_len,);

        if subproof_len % RECURSION_BRANCHOUT_NUM != 0 {
            let pad_num = RECURSION_BRANCHOUT_NUM - subproof_len % RECURSION_BRANCHOUT_NUM;
            tracing::info!("at level {}, {} subproofs are not a multiple of RECURSION_BRANCHOUT_NUM {}, hence padding {} empty proofs. ", level, subproof_len, RECURSION_BRANCHOUT_NUM, pad_num);

            last_level_proofs.resize(subproof_len + pad_num, last_level_empty_proof.clone());
        }

        let (recursive_circuit, recursive_targets) = circuit_registry
            .get_recursive_circuit(&last_level_vd_digest)
            .expect(format!("No recursive circuit found for level {}", level).as_str());

        let mut this_level_proofs = vec![];

        let _ = last_level_proofs
            .chunks(parallism * RECURSION_BRANCHOUT_NUM)
            .map(|chunk| {
                let proofs: Vec<ProofWithPublicInputs<F, C, D>> = chunk
                    .par_chunks(RECURSION_BRANCHOUT_NUM)
                    .map(|subproofs| {
                        let recursive_prover = RecursiveProver {
                            sub_proofs: subproofs
                                .to_owned()
                                .try_into()
                                .expect("subproofs length not equal to RECURSION_BRANCHOUT_NUM"),
                            sub_circuit_vd: last_level_circuit_vd.clone(),
                        };
                        let proof = recursive_prover.get_proof_with_circuit_data(
                            recursive_targets.clone(),
                            &recursive_circuit,
                        );
                        // TODO: consider valid proof and parse the tree node from the proof and check against the one generated by merkle sum tree.
                        proof
                    })
                    .collect();
                this_level_proofs.extend(proofs.into_iter());
            })
            .collect::<Vec<_>>();

        last_level_circuit_vd = recursive_circuit.verifier_only.clone();
        last_level_proofs = this_level_proofs;
        tracing::info!(
            "finish recursive level {} with {} proofs in : {:?}",
            level,
            last_level_proofs.len(),
            start.elapsed()
        );
    }

    if last_level_proofs.len() != 1 {
        panic!("The last level proofs should be of length 1, but got {}", last_level_proofs.len());
    }
    let root_proof = last_level_proofs.pop().unwrap();
    circuit_registry
        .get_root_circuit()
        .verify(root_proof.clone())
        .expect("fail to verify root proof");
    root_proof
}
