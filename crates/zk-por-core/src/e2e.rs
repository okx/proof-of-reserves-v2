use plonky2:: plonk::{
        proof::ProofWithPublicInputs,
        circuit_data::VerifierOnlyCircuitData,
};

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
                        prover.get_proof_with_circuit_data(account_targets.clone(), &batch_circuit);
                    proof
                })
                .collect();
            batch_proofs.extend(proofs.into_iter());
        })
        .collect::<Vec<_>>();

    batch_proofs
}

pub fn prove_subproofs<const RECURSION_BRANCHOUT_NUM: usize>(subproofs: Vec<ProofWithPublicInputs<F, C, D>>, last_level_circuit_vd : VerifierOnlyCircuitData<C, D>, circuit_registry: &CircuitRegistry<RECURSION_BRANCHOUT_NUM>, parallism : usize, level : usize) -> Vec<ProofWithPublicInputs<F, C, D>> {
    assert_eq!(subproofs.len() % RECURSION_BRANCHOUT_NUM, 0);
    let last_level_vd_digest = last_level_circuit_vd.circuit_digest;

    let (recursive_circuit, recursive_targets) = circuit_registry
        .get_recursive_circuit(&last_level_vd_digest)
        .expect(format!("No recursive circuit found for inner circuit with vd {:?}", last_level_vd_digest).as_str());

    let expected_this_level_proof_num = subproofs.len() / RECURSION_BRANCHOUT_NUM;
    let mut this_level_proofs = vec![];

    let _ = subproofs
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
                    proof
                })
                .collect();
            this_level_proofs.extend(proofs.into_iter());
            tracing::info!("finish {}/{} proofs in level {}/{}", this_level_proofs.len(), expected_this_level_proof_num, level, circuit_registry.get_recursive_levels());
        })
        .collect::<Vec<_>>();
    
    this_level_proofs
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

    for level in 1..=recursive_levels {
        let start = std::time::Instant::now();

        let last_level_vd_digest = last_level_circuit_vd.circuit_digest;
        let last_level_empty_proof =
            circuit_registry.get_empty_proof(&last_level_vd_digest).expect(
                format!(
                    "fail to find empty proof for circuit vd {:?}", last_level_vd_digest
                )
                .as_str(),
            );

        let subproof_len = last_level_proofs.len();

        if subproof_len % RECURSION_BRANCHOUT_NUM != 0 {
            let pad_num = RECURSION_BRANCHOUT_NUM - subproof_len % RECURSION_BRANCHOUT_NUM;
            tracing::info!("At level {}, {} subproofs are not a multiple of RECURSION_BRANCHOUT_NUM {}, hence padding {} empty proofs. ", level, subproof_len, RECURSION_BRANCHOUT_NUM, pad_num);

            last_level_proofs.resize(subproof_len + pad_num, last_level_empty_proof.clone());
        }

        last_level_proofs = prove_subproofs(last_level_proofs, last_level_circuit_vd.clone(), circuit_registry, parallism, level);

        let recursive_circuit = circuit_registry
            .get_recursive_circuit(&last_level_circuit_vd.circuit_digest)
            .expect(format!("No recursive circuit found for inner circuit with vd {:?}", last_level_circuit_vd.circuit_digest).as_str())
            .0;

        last_level_circuit_vd = recursive_circuit.verifier_only.clone();

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
