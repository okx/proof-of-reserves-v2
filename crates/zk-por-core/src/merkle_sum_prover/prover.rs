use crate::{
    account::Account,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::account_circuit::{AccountSumTargets, AccountTargets},
    types::{C, D, F},
};
use log::Level;
use plonky2::{
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder, circuit_data::CircuitData, proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};

use tracing::{error, info};

use super::circuits::merkle_sum_circuit::MerkleSumTreeTarget;

/// A merkle sum tree prover with a batch id representing its index in the recursive proof tree and a Vec of accounts representing accounts in this batch.
#[derive(Debug)]
pub struct MerkleSumTreeProver {
    // batch_id: usize,
    pub accounts: Vec<Account>,
}

impl MerkleSumTreeProver {
    /// Sets provided account targets with values from accounts in the prover batch.
    pub fn set_merkle_tree_targets(
        &self,
        pw: &mut PartialWitness<F>,
        account_targets: &Vec<AccountTargets>,
    ) {
        for i in 0..self.accounts.len() {
            // Set account targets
            account_targets.get(i).unwrap().set_account_targets(self.accounts.get(i).unwrap(), pw);
        }
    }

    /// Builds a merkle sum tree targets and returns the account targets to be set with input values.
    pub fn build_merkle_tree_targets(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Vec<AccountTargets> {
        let mut account_targets: Vec<AccountTargets> = Vec::new();

        for i in 0..self.accounts.len() {
            // Build account targets
            let account_target =
                AccountTargets::new_from_account(self.accounts.get(i).unwrap(), builder);
            // Set account targets
            account_targets.push(account_target);
        }

        let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
            .iter()
            .map(|x| AccountSumTargets::from_account_target(x, builder))
            .collect();

        // build merkle sum tree
        let _merkle_tree_targets =
            MerkleSumTreeTarget::build_new_from_account_targets(builder, &mut account_sum_targets);

        account_targets
    }

    /// Get the merkle sum tree proof of this batch of accounts.
    pub fn get_proof(&self) -> ProofWithPublicInputs<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        // Build and set merkle tree targets
        let account_targets = self.build_merkle_tree_targets(&mut builder);
        self.set_merkle_tree_targets(&mut pw, &account_targets);

        builder.print_gate_counts(0);

        let mut timing = TimingTree::new("prove", Level::Debug);
        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        info!("Started Proving");

        let proof_res = prove(&prover_only, &common, pw, &mut timing);

        match proof_res {
            Ok(proof) => {
                info!("Finished Proving");

                let proof_verification_res = data.verify(proof.clone());
                match proof_verification_res {
                    Ok(_) => proof,
                    Err(e) => {
                        error!("Proof verification failed: {:?}", e);
                        panic!("Proof verification failed!");
                    }
                }
            }
            Err(e) => {
                error!("Proof generation failed: {:?}", e);
                panic!("Proof generation failed!");
            }
        }
    }

    /// Get proof with a pre-compiled merkle sum circuit and account targets. In this method we do not need to build the circuit as we use a pre-built circuit.
    pub fn get_proof_with_circuit_data(
        &self,
        account_targets: &Vec<AccountTargets>,
        circuit_data: &CircuitData<F, C, D>,
    ) -> ProofWithPublicInputs<F, C, D> {
        let mut pw = PartialWitness::<F>::new();
        for i in 0..self.accounts.len() {
            // Build account targets
            let account_target = account_targets.get(i).unwrap();
            // Set account targets
            account_target.set_account_targets(self.accounts.get(i).unwrap(), &mut pw);
        }

        let mut timing = TimingTree::new("prove", Level::Info);

        let CircuitData { prover_only, common, verifier_only: _ } = &circuit_data;

        log::debug!("Starting proving!");

        let proof_res = prove(&prover_only, &common, pw, &mut timing);

        match proof_res {
            Ok(proof) => {
                log::debug!("Finished proving!");

                let proof_verification_res = circuit_data.verify(proof.clone());
                match proof_verification_res {
                    Ok(_) => proof,
                    Err(e) => {
                        error!("Proof verification failed: {:?}", e);
                        panic!("Proof verification failed!");
                    }
                }
            }
            Err(e) => {
                error!("Proof generation failed: {:?}", e);
                panic!("Proof generation failed!");
            }
        }
    }

    /// Get the merkle sum tree proof of this batch of accounts and the circuit data of the corresponding proof.
    pub fn get_proof_and_circuit_data(
        &self,
    ) -> (ProofWithPublicInputs<F, C, D>, CircuitData<F, C, D>) {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        // Build and set merkle tree targets
        let account_targets = self.build_merkle_tree_targets(&mut builder);
        self.set_merkle_tree_targets(&mut pw, &account_targets);

        builder.print_gate_counts(0);

        let mut timing = TimingTree::new("prove", Level::Debug);
        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        log::debug!("Starting proving!");

        let proof_res = prove(&prover_only, &common, pw, &mut timing);

        match proof_res {
            Ok(proof) => {
                log::debug!("Finished proving!");

                let proof_verification_res = data.verify(proof.clone());
                match proof_verification_res {
                    Ok(_) => (proof, data),
                    Err(e) => {
                        error!("Proof verification failed: {:?}", e);
                        panic!("Proof verification failed!");
                    }
                }
            }
            Err(e) => {
                error!("Proof generation failed: {:?}", e);
                panic!("Proof generation failed!");
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::{circuit_utils::run_circuit_test, parser::read_json_into_accounts_vec};

    use super::MerkleSumTreeProver;

    #[test]
    pub fn test_build_and_set_merkle_targets() {
        run_circuit_test(|builder, pw| {
            let path = "../../test-data/batch0.json";
            let accounts = read_json_into_accounts_vec(path);
            let prover = MerkleSumTreeProver {
                // batch_id: 0,
                accounts,
            };

            // Build and set merkle tree targets
            let account_targets = prover.build_merkle_tree_targets(builder);
            prover.set_merkle_tree_targets(pw, &account_targets);
        });
    }

    #[cfg(not(feature = "lightweight_test"))]
    #[test]
    pub fn test_get_proof() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);
        let prover = MerkleSumTreeProver {
            // batch_id: 0,
            accounts,
        };

        let _proof = prover.get_proof();
    }
}
