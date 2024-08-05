use log::Level;
use plonky2::{
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::CircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};
use tracing::error;

use crate::{
    circuit_config::STANDARD_CONFIG,
    types::{D, F},
};

use super::recursive_circuit::{verify_n_subproof_circuit, RecursiveTargets};

pub struct RecursiveProver<C: GenericConfig<D, F = F>, const N: usize> {
    // pub batch_id: usize,
    pub sub_proofs: [ProofWithPublicInputs<F, C, D>; N],
    pub merkle_sum_circuit: CircuitData<F, C, D>,
}

impl<C: GenericConfig<D, F = F>, const N: usize> RecursiveProver<C, N> {

    /// build recursive circuit that proves N subproofs and geneate parent merkle sum node targets
    /// This circuit hardcode the constraint that the verifier_circuit_target.circuit_digest must be equal to that inner_verifier_circuit_data.circuit_digest;
    pub fn build_new_recursive_n_circuit_targets(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> RecursiveTargets<N>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        // Verify n subproofs in circuit
        let mut recursive_targets =
            verify_n_subproof_circuit(builder, &self.merkle_sum_circuit.common, &self.merkle_sum_circuit.verifier_only);

        // Build the recursive merkle sum tree targets to get the next merkle sum tree root.
        recursive_targets.build_recursive_merkle_sum_tree_circuit(builder);

        #[cfg(debug_assertions)]
        builder.print_gate_counts(0);

        recursive_targets
    }


    pub fn build_and_set_recursive_targets(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        pw: &mut PartialWitness<F>,
    ) where
        <C as GenericConfig<2>>::Hasher: AlgebraicHasher<F>,
    {
        let recursive_targets: RecursiveTargets<N> = self.build_new_recursive_n_circuit_targets(
            builder,
        );

        recursive_targets.set_targets(
            pw,
            self.sub_proofs.to_vec(),
            &self.merkle_sum_circuit.verifier_only,
        )
    }

    /// Gets the proof with pis of this batch of recursive proofs.
    pub fn get_proof(&self) -> ProofWithPublicInputs<F, C, D>
    where
        <C as GenericConfig<2>>::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let mut pw = PartialWitness::<F>::new();

        self.build_and_set_recursive_targets(&mut builder, &mut pw);

        builder.print_gate_counts(0);

        let mut timing = TimingTree::new("prove", Level::Info);
        let data = builder.build::<C>();

        let CircuitData { prover_only, common, verifier_only: _ } = &data;

        log::debug!("before prove");
        let start = std::time::Instant::now();

        let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);

        log::debug!("time for {:?} proofs, {:?}", N, start.elapsed().as_millis());

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

    /// Get proof with a pre-compiled merkle sum circuit and recursive targets. In this method we do not need to build the circuit as we use a pre-built circuit.
    pub fn get_proof_with_circuit_data(&self, recursive_targets: &RecursiveTargets<N>, cd: &CircuitData<F, C, D>) -> ProofWithPublicInputs<F, C, D>
    where
        <C as GenericConfig<2>>::Hasher: AlgebraicHasher<F>,
    {
        let mut pw = PartialWitness::<F>::new();
        let CircuitData { prover_only, common, verifier_only } = &cd;

        recursive_targets.set_targets(&mut pw, self.sub_proofs.to_vec(), verifier_only);

        let mut timing = TimingTree::new("prove", Level::Info);

        log::debug!("before prove");
        let start = std::time::Instant::now();

        let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);

        log::debug!("time for {:?} proofs, {:?}", N, start.elapsed().as_millis());

        match proof_res {
            Ok(proof) => {
                println!("Finished Proving");

                let proof_verification_res = cd.verify(proof.clone());
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