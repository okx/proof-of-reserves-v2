use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::{
        CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
    },
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputsTarget,
};

use crate::{merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget, recursive::recursive_circuit_utils::verify_n_subproof_circuit};

use crate::types::{C, D, F};

/// Struct representing the targets of a recusive circuit. Since we have the same type of subproofs, we only need one type of verifier circuit as
/// we can verify all the targets using the same circuit.
#[derive(Clone)]
pub struct RecursiveTargets<const N: usize> {
    pub proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>>,
    pub verifier_circuit_target: VerifierCircuitTarget, // Only one needed instead of N
}

/// build recursive circuit that proves N subproofs and geneate parent merkle sum node targets
/// This circuit hardcode the constraint that the verifier_circuit_target.circuit_digest must be equal to that inner_verifier_circuit_data.circuit_digest;
pub fn build_new_recursive_n_circuit_targets<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    inner_common_circuit_data: &CommonCircuitData<F, D>,
    inner_verifier_circuit_data: &VerifierOnlyCircuitData<InnerC, D>,
    builder: &mut CircuitBuilder<F, D>,
) -> RecursiveTargets<N>
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    // Verify n subproofs in circuit
    let verification_targets = verify_n_subproof_circuit(builder, inner_common_circuit_data, inner_verifier_circuit_data);

    let mut merkle_sum_node_targets: Vec<MerkleSumNodeTarget> = Vec::new();
    // TODO: clone is NOT safe.
    merkle_sum_node_targets.push(MerkleSumNodeTarget::from(verification_targets.proof_with_pis_targets[0].public_inputs.clone()));
        
    (1..N).for_each(|i| {
        merkle_sum_node_targets[i] = MerkleSumNodeTarget::get_parent_from_children(
            &mut builder,
            &merkle_sum_node_targets[i - 1],
            &MerkleSumNodeTarget::from(proof_with_pub_input_targets[i].public_inputs.clone()),
        );
    });
    merkle_sum_node_targets[N - 1].registered_as_public_inputs(&mut builder);

    #[cfg(debug_assertions)]
    builder.print_gate_counts(0);

    tracing::debug!("before build recurisve {} circuit", N);
    let circuit_data = builder.build::<C>();
    tracing::debug!("after build recurisve {} circuit", N);

    (
        circuit_data,
        RecursiveTargets::<N> {
            proof_with_pub_input_targets: proof_with_pub_input_targets,
            verifier_circuit_target: verifier_circuit_target,
        },
    )
}
