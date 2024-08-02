use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::{
        CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
    },
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputsTarget,
};

use plonky2::iop::target::Target;

use crate::{
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget,
};

use crate::types::{C, D, F};

#[derive(Clone)]
pub struct RecursiveTargets<const N: usize> {
    pub proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>>,
    pub verifier_circuit_target: VerifierCircuitTarget,
}

/// build recursive circuit that proves N subproofs and geneate parent merkle sum node targets
// This circuit hardcode the constraint that the verifier_circuit_target.circuit_digest must be equal to that inner_verifier_circuit_data.circuit_digest;
pub fn build_recursive_n_circuit<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    inner_common_circuit_data: &CommonCircuitData<F, D>,
    inner_verifier_circuit_data: &VerifierOnlyCircuitData<InnerC, D>,
) -> (CircuitData<F, C, D>, RecursiveTargets<N>)
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let verifier_circuit_target = VerifierCircuitTarget {
        constants_sigmas_cap: builder
            .add_virtual_cap(inner_common_circuit_data.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };

    let vd_digest = inner_verifier_circuit_data.circuit_digest;
    let vd_digest_target = builder.constant_hash(vd_digest);
    builder.connect_hashes(verifier_circuit_target.circuit_digest, vd_digest_target);

    let mut proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>> = vec![];
    (0..N).for_each(|_| {
        let proof_with_pub_input_target =
            builder.add_virtual_proof_with_pis::<InnerC>(inner_common_circuit_data);
        builder.verify_proof::<InnerC>(
            &proof_with_pub_input_target,
            &verifier_circuit_target,
            inner_common_circuit_data,
        );
        proof_with_pub_input_targets.push(proof_with_pub_input_target);
    });

    // for each pub_input target from proof_with_pub_input_targets, convert it to MerkleSumNodeTarget, contraint them to parent merkle sum node target, and then convert them back to pub_input target.
    let mut merkle_sum_tree_node_targets = [MerkleSumNodeTarget::default(); N];
    (0..N).for_each(|i| {
        let targets = std::mem::take(&mut proof_with_pub_input_targets[i].public_inputs);
        merkle_sum_tree_node_targets[i] = MerkleSumNodeTarget::from(targets);
    });

    let parent_merkle_sum_node_target = MerkleSumNodeTarget::get_parent_from_children::<N>(
        &mut builder,
        merkle_sum_tree_node_targets.iter().collect::<Vec<_>>().try_into().unwrap(),
    );

    (0..N).for_each(|i| {
        let public_input_target = Vec::<Target>::from(merkle_sum_tree_node_targets[i]);
        proof_with_pub_input_targets[i].public_inputs = public_input_target;
    });

    parent_merkle_sum_node_target.registered_as_public_inputs(&mut builder);

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
