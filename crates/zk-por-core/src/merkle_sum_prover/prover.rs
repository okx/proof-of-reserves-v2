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

use tracing::error;

use super::circuits::merkle_sum_circuit::MerkleSumTreeTarget;

/// A merkle sum tree prover with a batch id representing its index in the recursive proof tree and a Vec of accounts representing accounts in this batch.
#[derive(Clone, Debug)]
pub struct MerkleSumTreeProver {
    // batch_id: usize,
    pub accounts: Vec<Account>,
}

impl MerkleSumTreeProver {
    /// Build the merkle sum tree targets and set the account targets with the account info.
    pub fn build_and_set_merkle_tree_targets(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        pw: &mut PartialWitness<F>,
    ) {
        let mut account_targets: Vec<AccountTargets> = Vec::new();

        for i in 0..self.accounts.len() {
            // Build account targets
            let account_target =
                AccountTargets::new_from_account(self.accounts.get(i).unwrap(), builder);
            // Set account targets
            account_target.set_account_targets(self.accounts.get(i).unwrap(), pw);
            account_targets.push(account_target);
        }

        let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
            .iter()
            .map(|x| AccountSumTargets::from_account_target(x, builder))
            .collect();

        // build merkle sum tree
        let _merkle_tree_targets =
            MerkleSumTreeTarget::build_new_from_account_targets(builder, &mut account_sum_targets);
    }

    pub fn get_prover_cd(&self, builder: &mut CircuitBuilder<F, D>) -> CircuitData<F, C, D> {
        let mut account_targets: Vec<AccountTargets> = Vec::new();

        for i in 0..self.accounts.len() {
            // Build account targets
            let account_target =
                AccountTargets::new_from_account(self.accounts.get(i).unwrap(), builder);
            account_targets.push(account_target);
        }

        let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
            .iter()
            .map(|x| AccountSumTargets::from_account_target(x, builder))
            .collect();

        // build merkle sum tree
        let _merkle_tree_targets =
            MerkleSumTreeTarget::build_new_from_account_targets(builder, &mut account_sum_targets);

        builder.clone().build::<C>()
    }

    /// Get the merkle sum tree proof of this batch of accounts.
    pub fn get_proof(&self) -> ProofWithPublicInputs<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        self.build_and_set_merkle_tree_targets(&mut builder, &mut pw);

        builder.print_gate_counts(0);

        let mut timing = TimingTree::new("prove", Level::Debug);
        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        println!("Started Proving");

        let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);

        match proof_res {
            Ok(proof) => {
                println!("Finished Proving");

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

    /// Get the merkle sum tree proof of this batch of accounts.
    pub fn get_proof_with_cd(&self) -> (ProofWithPublicInputs<F, C, D>, CircuitData<F, C, D>) {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        self.build_and_set_merkle_tree_targets(&mut builder, &mut pw);

        builder.print_gate_counts(0);

        let mut timing = TimingTree::new("prove", Level::Debug);
        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        println!("Started Proving");

        let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);

        match proof_res {
            Ok(proof) => {
                println!("Finished Proving");

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

            prover.build_and_set_merkle_tree_targets(builder, pw);
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
