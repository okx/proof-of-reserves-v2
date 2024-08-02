use plonky2::{
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget},
    },
};

use crate::merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget;

use crate::types::{D, F};

/// Struct representing the targets of a recusive circuit. Since we have the same type of subproofs, we only need one type of verifier circuit as
/// we can verify all the targets using the same circuit.
#[derive(Clone)]
pub struct RecursiveTargets<const N: usize> {
    pub proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>>,
    pub verifier_circuit_target: VerifierCircuitTarget, // Only one needed instead of N
}

impl<const N: usize> RecursiveTargets<N> {
    /// Builds a N-ary merkle sum tree and sets its root as a public input.
    pub fn build_recursive_merkle_sum_tree_circuit(&mut self, builder: &mut CircuitBuilder<F, D>) {
        let mut merkle_sum_node_targets: Vec<MerkleSumNodeTarget> = Vec::new();
        merkle_sum_node_targets.push(MerkleSumNodeTarget::from(
            self.proof_with_pub_input_targets[0].public_inputs.clone(),
        ));
        let mut merkle_sum_tree_node_targets: Vec<MerkleSumNodeTarget> = Vec::new();

        (0..N).for_each(|i| {
            let targets = std::mem::take(&mut self.proof_with_pub_input_targets[i].public_inputs);
            merkle_sum_tree_node_targets.push(MerkleSumNodeTarget::from(targets));
        });

        let parent_merkle_sum_node_target = MerkleSumNodeTarget::get_parent_from_children::<N>(
            builder,
            &merkle_sum_tree_node_targets,
        );

        (0..N).for_each(|i| {
            let public_input_target = Vec::<Target>::from(merkle_sum_tree_node_targets[i]);
            self.proof_with_pub_input_targets[i].public_inputs = public_input_target;
        });

        parent_merkle_sum_node_target.register_as_public_input(builder);
    }

    /// Sets recursive targets with values from subproof PIs and the verifier cd.
    pub fn set_targets<C: GenericConfig<D, F = F>>(
        &self,
        pw: &mut PartialWitness<F>,
        sub_proofs: Vec<ProofWithPublicInputs<F, C, D>>,
        inner_circuit_vd: &VerifierOnlyCircuitData<C, D>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        pw.set_verifier_data_target(&self.verifier_circuit_target, inner_circuit_vd);

        (0..N).for_each(|i| {
            pw.set_proof_with_pis_target(&self.proof_with_pub_input_targets[i], &sub_proofs[i]);
        });
    }
}

/// build recursive circuit that proves N subproofs and geneate parent merkle sum node targets
/// This circuit hardcode the constraint that the verifier_circuit_target.circuit_digest must be equal to that inner_verifier_circuit_data.circuit_digest;
pub fn build_new_recursive_n_circuit_targets<C: GenericConfig<D, F = F>, const N: usize>(
    inner_common_circuit_data: &CommonCircuitData<F, D>,
    inner_verifier_circuit_data: &VerifierOnlyCircuitData<C, D>,
    builder: &mut CircuitBuilder<F, D>,
) -> RecursiveTargets<N>
where
    C::Hasher: AlgebraicHasher<F>,
{
    // Verify n subproofs in circuit
    let mut recursive_targets =
        verify_n_subproof_circuit(builder, inner_common_circuit_data, inner_verifier_circuit_data);

    // Build the recursive merkle sum tree targets to get the next merkle sum tree root.
    recursive_targets.build_recursive_merkle_sum_tree_circuit(builder);

    #[cfg(debug_assertions)]
    builder.print_gate_counts(0);

    recursive_targets
}

pub fn verify_n_subproof_circuit<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    subproof_common_cd: &CommonCircuitData<F, D>,
    subproof_verifier_cd: &VerifierOnlyCircuitData<InnerC, D>,
) -> RecursiveTargets<N>
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    let verifier_circuit_targets = VerifierCircuitTarget {
        constants_sigmas_cap: builder
            .add_virtual_cap(subproof_common_cd.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };

    let vd_digest = subproof_verifier_cd.circuit_digest;
    let vd_digest_target = builder.constant_hash(vd_digest);
    builder.connect_hashes(verifier_circuit_targets.circuit_digest, vd_digest_target);

    // _inner_verifier_circuit_data.circuit_digest;
    let mut proof_with_pis_targets: Vec<ProofWithPublicInputsTarget<D>> = vec![];
    (0..N).for_each(|_| {
        let proof_with_pub_input_target =
            builder.add_virtual_proof_with_pis::<InnerC>(subproof_common_cd);
        builder.verify_proof::<InnerC>(
            &proof_with_pub_input_target,
            &verifier_circuit_targets,
            subproof_common_cd,
        );
        proof_with_pis_targets.push(proof_with_pub_input_target);
    });

    RecursiveTargets {
        verifier_circuit_target: verifier_circuit_targets,
        proof_with_pub_input_targets: proof_with_pis_targets,
    }
}
