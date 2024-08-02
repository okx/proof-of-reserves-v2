use plonky2::plonk::{circuit_data::CircuitData, proof::ProofWithPublicInputs};

use crate::{
    account::Account,
    // circuit_config::{BATCH_SIZE, RECURSIVE_FACTOR, ASSET_NUM},
    merkle_sum_prover::{
        circuits::{
            account_circuit::AccountTargets, merkle_sum_circuit::build_merkle_sum_tree_circuit,
        },
        prover::MerkleSumTreeProver,
    },
    recursive::circuit::{build_recursive_n_circuit, RecursiveTargets},
    recursive::prove::prove_n_subproofs,
};

use plonky2_field::types::Field;
use rand::Rng;

use crate::types::{C, D, F};
// use once_cell::sync::Lazy;

pub struct CircuitRegistry<const RECURSIVE_FACTOR: usize> {
    batch_circuit: (CircuitData<F, C, D>, Vec<AccountTargets>),
    empty_batch_proof: ProofWithPublicInputs<F, C, D>,
    recursive_circuits_and_empty_proofs: Vec<(
        (CircuitData<F, C, D>, RecursiveTargets<RECURSIVE_FACTOR>),
        ProofWithPublicInputs<F, C, D>,
    )>,
}

fn gen_empty_accounts(num_accounts: usize, num_assets: usize) -> (Vec<Account>, u32, u32) {
    let mut accounts: Vec<Account> = Vec::new();
    let mut rng = rand::thread_rng(); // Create a random number generator
    let mut equity_sum = 0;
    let mut debt_sum = 0;
    for _ in 0..num_accounts {
        let mut equities = Vec::new();
        let mut debts = Vec::new();
        for _ in 0..num_assets {
            let equity = 0;
            let debt = 0;
            equity_sum += equity;
            debt_sum += debt;
            equities.push(F::from_canonical_u32(equity));
            debts.push(F::from_canonical_u32(debt));
        }
        let account_id = rng.gen_range(0..i32::MAX).to_string();
        accounts.push(Account { id: account_id, equity: equities, debt: debts });
    }
    (accounts, equity_sum, debt_sum)
}

impl<const RECURSIVE_FACTOR: usize> CircuitRegistry<RECURSIVE_FACTOR> {
    pub fn init(batch_size: usize, asset_num: usize) -> Self {
        let init_start = std::time::Instant::now();

        let start = std::time::Instant::now();
        let (batch_circuit_data, account_targets) =
            build_merkle_sum_tree_circuit(batch_size, asset_num);
        tracing::info!(
            "build merkle sum tree circuit with batch size {} in : {:?}",
            batch_size,
            start.elapsed()
        );

        let accounts = gen_empty_accounts(batch_size, asset_num).0;

        let start = std::time::Instant::now();
        let prover = MerkleSumTreeProver { accounts: accounts };
        let empty_batch_proof =
            prover.prove_with_circuit(&batch_circuit_data, account_targets.clone()).unwrap();
        tracing::info!(
            "prove merkle sum tree with batch size {} in : {:?}",
            batch_size,
            start.elapsed()
        );

        let mut recursive_circuit_and_empty_proofs = Vec::new();

        let mut last_circuit_data = &batch_circuit_data;

        let mut level = 0;

        // keep building recursive circuits and proofs until the circuit is the same as the last level
        loop {
            let start = std::time::Instant::now();
            let (recursive_circuit, recursive_account_targets) =
                build_recursive_n_circuit::<C, RECURSIVE_FACTOR>(
                    &batch_circuit_data.common,
                    &batch_circuit_data.verifier_only,
                );
            tracing::info!("build recursive circuit at level {} in : {:?}", level, start.elapsed());

            let start = std::time::Instant::now();
            let recursive_proof = prove_n_subproofs(
                vec![empty_batch_proof.clone(); RECURSIVE_FACTOR],
                &last_circuit_data.verifier_only,
                &recursive_circuit,
                recursive_account_targets.clone(),
            )
            .unwrap();
            tracing::info!(
                "prove recursive subproofs at level {} in : {:?}",
                level,
                start.elapsed()
            );
            if recursive_circuit.verifier_only.circuit_digest
                == last_circuit_data.verifier_only.circuit_digest
            {
                break;
            }

            recursive_circuit_and_empty_proofs
                .push(((recursive_circuit, recursive_account_targets), recursive_proof));

            level += 1;
            last_circuit_data = &recursive_circuit_and_empty_proofs.last().unwrap().0 .0;
        }

        tracing::info!(
            "finish init circuit registry with {} recursive levels in {:?}",
            level,
            init_start.elapsed()
        );

        Self {
            batch_circuit: (batch_circuit_data, account_targets),
            empty_batch_proof: empty_batch_proof,
            recursive_circuits_and_empty_proofs: recursive_circuit_and_empty_proofs,
        }
    }

    pub fn get_batch_circuit(&self) -> (&CircuitData<F, C, D>, Vec<AccountTargets>) {
        (&self.batch_circuit.0, self.batch_circuit.1.clone())
    }

    pub fn get_empty_batch_circuit_proof(&self) -> ProofWithPublicInputs<F, C, D> {
        self.empty_batch_proof.clone()
    }

    /// leaf node at level 0
    /// if level is > 0, we return the last level circuit. It is because recursive circuit will be the same after several levels, though I dont know why...
    pub fn get_recursive_circuit(
        &self,
        level: usize,
    ) -> (&CircuitData<F, C, D>, RecursiveTargets<RECURSIVE_FACTOR>) {
        let mut circuit_and_empty_proof = self.recursive_circuits_and_empty_proofs.last().unwrap();
        if level < self.recursive_circuits_and_empty_proofs.len() {
            circuit_and_empty_proof = self.recursive_circuits_and_empty_proofs.get(level).unwrap();
        }
        (&circuit_and_empty_proof.0 .0, circuit_and_empty_proof.0 .1.clone())
    }

    pub fn get_empty_recursive_circuit_proof(
        &self,
        level: usize,
    ) -> ProofWithPublicInputs<F, C, D> {
        let mut circuit_and_empty_proof = self.recursive_circuits_and_empty_proofs.last().unwrap();
        if level < self.recursive_circuits_and_empty_proofs.len() {
            circuit_and_empty_proof = self.recursive_circuits_and_empty_proofs.get(level).unwrap();
        }
        circuit_and_empty_proof.1.clone()
    }
}

// TODO: this step is slow. Currently avoid calling.
// static REGISTRY: Lazy<CircuitRegistry> = Lazy::new(|| {
// 	CircuitRegistry::init()
// });
