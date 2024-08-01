use log::Level;
use plonky2::{iop::witness::PartialWitness, plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData, proof::ProofWithPublicInputs, prover::prove}, util::timing::TimingTree};

use crate::{
    circuit_config::STANDARD_CONFIG, merkle_sum_prover::circuits::account_circuit::AccountTargets, types::{C, D, F}
};

use super::{merkle_proof::MerkleProofProvingInputs, merkle_proof_circuits::MerkleProofTargets};

use tracing::error;

pub struct MerkleProofProver {
    pub merkle_proof: MerkleProofProvingInputs,
}

impl MerkleProofProver {
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

        account_targets.set_account_targets(&self.merkle_proof.account, pw);
    }

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
