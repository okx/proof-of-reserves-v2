use log::Level;
use plonky2::{
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder, circuit_data::CircuitData, proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};

use crate::{
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::account_circuit::AccountTargets,
    types::{C, D, F},
};

use super::{merkle_proof::MerkleProofProvingInputs, merkle_proof_circuits::MerkleProofTargets};

use tracing::error;

pub struct MerkleProofProver {
    pub merkle_proof: MerkleProofProvingInputs,
}

impl MerkleProofProver {
    /// Build the circuit for a merkle proof of inclusion of a given account at a specific index.
    pub fn build_and_set_merkle_proof_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        pw: &mut PartialWitness<F>,
    ) {
        let merkle_proof_len = self.merkle_proof.siblings.len();

        let account_targets = AccountTargets::new_from_account(&self.merkle_proof.account, builder);

        let merkle_proof_targets = MerkleProofTargets::new_from_account_targets(
            builder,
            &account_targets,
            merkle_proof_len,
        );

        merkle_proof_targets.verify_merkle_proof_circuit(builder);
        merkle_proof_targets.set_merkle_proof_targets(&self.merkle_proof, pw);

        account_targets.set_account_targets(&self.merkle_proof.account, pw);
    }

    /// Get the Proof with PI's of a merkle proof of inclusion of a users account at a given index.
    pub fn get_proof(&self) -> ProofWithPublicInputs<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        self.build_and_set_merkle_proof_circuit(&mut builder, &mut pw);

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
}

#[cfg(test)]
pub mod test {
    use crate::{
        circuit_utils::run_circuit_test,
        merkle_proof_prover::{merkle_proof::MerkleProofProvingInputs, prover::MerkleProofProver},
        merkle_sum_prover::merkle_sum_tree::MerkleSumTree,
        parser::read_json_into_accounts_vec,
    };

    #[test]
    pub fn test_build_and_set_merkle_targets() {
        run_circuit_test(|builder, pw| {
            let path = "../../test-data/batch0.json";
            let accounts = read_json_into_accounts_vec(path);
            let account = accounts.get(0).unwrap();
            let merkle_tree = MerkleSumTree::new_tree_from_accounts(&accounts);
            let merkle_proof_pis =
                MerkleProofProvingInputs::new_from_merkle_tree(0, account, &merkle_tree);
            let prover = MerkleProofProver { merkle_proof: merkle_proof_pis };

            prover.build_and_set_merkle_proof_circuit(builder, pw);
        });
    }

    #[test]
    pub fn test_get_proof() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);
        let account = accounts.get(0).unwrap();
        let merkle_tree = MerkleSumTree::new_tree_from_accounts(&accounts);
        let merkle_proof_pis =
            MerkleProofProvingInputs::new_from_merkle_tree(0, account, &merkle_tree);

        let prover = MerkleProofProver { merkle_proof: merkle_proof_pis };

        let _proof = prover.get_proof();
    }
}
