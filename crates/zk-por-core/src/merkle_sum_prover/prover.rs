use crate::{
    account::Account,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::{
        account_circuit::{AccountSumTargets, AccountTargets},
        merkle_sum_circuit::build_merkle_sum_tree_from_account_targets,
    },
    types::{C, D, F},
};
use log::Level;
use plonky2::{
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder, circuit_data::CircuitData, 
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::goldilocks_field::GoldilocksField;
use tracing::error;
use anyhow::Result;


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
            let equity_targets =
                builder.add_virtual_targets(self.accounts.get(i).unwrap().equity.len());
            let debt_targets =
                builder.add_virtual_targets(self.accounts.get(i).unwrap().debt.len());
            let account_target = AccountTargets { equity: equity_targets, debt: debt_targets };

            account_target.set_account_targets(self.accounts.get(i).unwrap(), pw);
            account_targets.push(account_target);
        }

        let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
            .iter()
            .map(|x| AccountSumTargets::from_account_target(x, builder))
            .collect();

        let _merkle_tree_targets =
            build_merkle_sum_tree_from_account_targets(builder, &mut account_sum_targets);
    }

    /// Get the merkle sum tree proof of this batch of accounts.
    pub fn get_proof(&self) -> ProofWithPublicInputs<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<GoldilocksField>::new();

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

    pub fn prove_with_circuit(&self, circuit_data : &CircuitData<F, C, D>, account_targets : Vec<AccountTargets>)-> Result<ProofWithPublicInputs<F, C, D>> {
        if account_targets.len() != self.accounts.len() {
            return Err(anyhow::anyhow!("Account targets length does not match accounts length"));
        }

        let mut pw = PartialWitness::new();
 
        let CircuitData { prover_only, common, verifier_only: _ } = circuit_data;

        for i in 0..self.accounts.len() {
            account_targets[i].set_account_targets(self.accounts.get(i).unwrap(), &mut pw);
        }

        let mut timing = TimingTree::new("prove_merkle_sum_tree", Level::Debug);
        let proof = prove(&prover_only, &common, pw, &mut timing)?;

        #[cfg(debug_assertions)]
        circuit_data.verify(proof.clone()).unwrap();

        Ok(proof)
    }
}

#[cfg(test)]
pub mod test {
    use log::Level;
    use plonky2::{
        iop::witness::PartialWitness,
        plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData, prover::prove},
        util::timing::TimingTree,
    };
    use plonky2_field::goldilocks_field::GoldilocksField;

    use crate::{
        circuit_config::STANDARD_CONFIG,
        parser::read_json_into_accounts_vec,
        types::{C, D, F},
        account::{gen_accounts_with_random_data},
        merkle_sum_prover::circuits::{
            merkle_sum_circuit::build_merkle_sum_tree_circuit,
        },
    };

    use super::MerkleSumTreeProver;
    use plonky2_field::types::Field;

    #[test]
    pub fn test_build_and_set_merkle_targets() {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<GoldilocksField>::new();

        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);
        let prover = MerkleSumTreeProver {
            // batch_id: 0,
            accounts,
        };

        prover.build_and_set_merkle_tree_targets(&mut builder, &mut pw);

        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        println!("Started Proving");
        let mut timing = TimingTree::new("prove", Level::Debug);
        let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);
        let proof = proof_res.expect("Proof failed");

        println!("Verifying Proof");
        // Verify proof
        let _proof_verification_res = data.verify(proof.clone()).unwrap();
    }

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



    #[test]
    pub fn test_separate_circuit_building_and_proving() {
        let num_accounts = 10;
        let num_assets = 5;
        let (circuit_data, account_targets) = build_merkle_sum_tree_circuit(num_accounts, num_assets);

        let (accounts, equity_sum, debt_sum) = gen_accounts_with_random_data(num_accounts, num_assets);
        let prover = MerkleSumTreeProver {
            accounts,
        };
        let proof_result = prover.prove_with_circuit(&circuit_data, account_targets);
        assert!(proof_result.is_ok());
        let proof = proof_result.unwrap();

        // account_sum and debt_sum are the public inputs
        assert_eq!(F::from_canonical_u32(equity_sum), proof.public_inputs[0]);
        assert_eq!(F::from_canonical_u32(debt_sum), proof.public_inputs[1]);
        assert!(circuit_data.verify(proof).is_ok());
    }
}
