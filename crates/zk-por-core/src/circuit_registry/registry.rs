use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::{CircuitConfig, CircuitData},
        proof::ProofWithPublicInputs,
    },
};

use crate::{
    account::gen_empty_accounts,
    // circuit_config::{BATCH_SIZE, RECURSION_BRANCHOUT_NUM, ASSET_NUM},
    merkle_sum_prover::{
        circuits::{
            account_circuit::AccountTargets, merkle_sum_circuit::build_merkle_sum_tree_circuit,
        },
        prover::MerkleSumTreeProver,
    },
    recursive_prover::prover::RecursiveProver,
    recursive_prover::recursive_circuit::{build_recursive_n_circuit, RecursiveTargets},
    types::{C, D, F},
};

use std::collections::HashMap;

#[allow(clippy::type_complexity)]
pub struct CircuitRegistry<const RECURSION_BRANCHOUT_NUM: usize> {
    batch_circuit: (CircuitData<F, C, D>, Vec<AccountTargets>),
    // inner_vd => the verification circuit that verify the inner circuit
    recursive_circuits:
        HashMap<HashOut<F>, (CircuitData<F, C, D>, RecursiveTargets<RECURSION_BRANCHOUT_NUM>)>,
    // circuit_vd -> empty proof
    empty_proofs: HashMap<HashOut<F>, ProofWithPublicInputs<F, C, D>>,
    last_inner_circuit_vd: HashOut<F>,
}

impl<const RECURSION_BRANCHOUT_NUM: usize> CircuitRegistry<RECURSION_BRANCHOUT_NUM> {
    pub fn init(
        batch_size: usize,
        asset_num: usize,
        batch_circuit_config: CircuitConfig,
        recursive_level_configs: Vec<CircuitConfig>,
    ) -> Self {
        let init_start = std::time::Instant::now();

        let start = std::time::Instant::now();
        let (batch_circuit_data, account_targets) =
            build_merkle_sum_tree_circuit(batch_size, asset_num, batch_circuit_config);
        tracing::info!(
            "build merkle sum tree circuit with batch size {} in : {:?}",
            batch_size,
            start.elapsed()
        );

        let accounts = gen_empty_accounts(batch_size, asset_num);

        let start = std::time::Instant::now();
        let prover = MerkleSumTreeProver { accounts };
        let empty_batch_proof =
            prover.get_proof_with_circuit_data(&account_targets, &batch_circuit_data);
        tracing::info!(
            "prove merkle sum tree with batch size {} in : {:?}",
            batch_size,
            start.elapsed()
        );

        let mut empty_proofs = HashMap::new();
        let mut recursive_circuits = HashMap::new();
        let mut last_empty_proof = empty_batch_proof.clone();

        let mut last_circuit_data = &batch_circuit_data;
        let mut last_circuit_vd = last_circuit_data.verifier_only.circuit_digest;
        empty_proofs
            .insert(last_circuit_data.verifier_only.circuit_digest, last_empty_proof.clone());

        for (level, circuit_config) in recursive_level_configs.into_iter().enumerate() {
            let start = std::time::Instant::now();
            let (recursive_circuit, recursive_targets) =
                build_recursive_n_circuit::<C, RECURSION_BRANCHOUT_NUM>(
                    &last_circuit_data.common,
                    &last_circuit_data.verifier_only,
                    circuit_config,
                );
            tracing::info!(
                "build recursive circuit at level {} in : {:?}, with vd {:?}",
                level,
                start.elapsed(),
                recursive_circuit.verifier_only.circuit_digest
            );
            let sub_proofs: [ProofWithPublicInputs<F, C, D>; RECURSION_BRANCHOUT_NUM] =
                std::array::from_fn(|_| last_empty_proof.clone());
            let start = std::time::Instant::now();
            let recursive_prover = RecursiveProver {
                sub_proofs,
                sub_circuit_vd: last_circuit_data.verifier_only.clone(),
            };
            let recursive_proof = recursive_prover
                .get_proof_with_circuit_data(recursive_targets.clone(), &recursive_circuit);

            tracing::info!(
                "prove recursive subproofs at level {} in : {:?}",
                level,
                start.elapsed()
            );

            empty_proofs
                .insert(recursive_circuit.verifier_only.circuit_digest, recursive_proof.clone());

            last_circuit_vd = last_circuit_data.verifier_only.circuit_digest;
            recursive_circuits.insert(last_circuit_vd, (recursive_circuit, recursive_targets));

            last_circuit_data = &recursive_circuits[&last_circuit_vd].0;
            last_empty_proof = recursive_proof;
        }

        tracing::info!(
            "finish init circuit registry with {} recursive levels in {:?}",
            recursive_circuits.len(),
            init_start.elapsed()
        );

        Self {
            batch_circuit: (batch_circuit_data, account_targets),
            empty_proofs: empty_proofs,
            recursive_circuits: recursive_circuits,
            last_inner_circuit_vd: last_circuit_vd,
        }
    }

    pub fn get_batch_circuit(&self) -> (&CircuitData<F, C, D>, &[AccountTargets]) {
        (&self.batch_circuit.0, &self.batch_circuit.1)
    }

    pub fn get_empty_proof(
        &self,
        circuit_vd: &HashOut<F>,
    ) -> Option<&ProofWithPublicInputs<F, C, D>> {
        self.empty_proofs.get(circuit_vd)
    }

    /// leaf node at level 0
    pub fn get_recursive_circuit(
        &self,
        inner_circuit_vd: &HashOut<F>,
    ) -> Option<(&CircuitData<F, C, D>, &RecursiveTargets<RECURSION_BRANCHOUT_NUM>)> {
        let circuit_and_targets = self.recursive_circuits.get(inner_circuit_vd)?;
        Some((&circuit_and_targets.0, &circuit_and_targets.1))
    }

    pub fn get_recursive_levels(&self) -> usize {
        self.recursive_circuits.len()
    }

    pub fn get_root_circuit(&self) -> &CircuitData<F, C, D> {
        &self.recursive_circuits[&self.last_inner_circuit_vd].0
    }
}
