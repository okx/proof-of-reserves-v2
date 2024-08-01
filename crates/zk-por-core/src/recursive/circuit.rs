use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::{
        CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
    },
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputsTarget,
};

use crate::{
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget,
};

use crate::types::{C, D, F};

pub struct RecursiveTargets<const N: usize> {
    pub size: usize,
    pub proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>>,
    pub verifier_circuit_target: VerifierCircuitTarget,
    // pub vd_proof_target: VdProofTarget,
}

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
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG.clone());
    let verifier_circuit_target = VerifierCircuitTarget {
        constants_sigmas_cap: builder
            .add_virtual_cap(inner_common_circuit_data.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };

    // hardcode the constraint the verifier_circuit_target.circuit_digest = _inner_verifier_circuit_data.circuit_digest;
    let vd_digest = inner_verifier_circuit_data.circuit_digest;
    let vd_digest_target = builder.constant_hash(vd_digest);
    builder.connect_hashes(verifier_circuit_target.circuit_digest, vd_digest_target);

    // _inner_verifier_circuit_data.circuit_digest;
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

    let mut merkle_sum_node_targets: [MerkleSumNodeTarget; N] = [MerkleSumNodeTarget::default(); N];
    merkle_sum_node_targets[0] =
        MerkleSumNodeTarget::from(proof_with_pub_input_targets[0].public_inputs.clone());
    (1..N).for_each(|i| {
        merkle_sum_node_targets[i] = MerkleSumNodeTarget::get_child_from_parents(
            &mut builder,
            &merkle_sum_node_targets[i - 1],
            &MerkleSumNodeTarget::from(proof_with_pub_input_targets[i].public_inputs.clone()),
        );
    });
    merkle_sum_node_targets[N - 1].registered_as_public_inputs(&mut builder);

    #[cfg(debug_assertions)]
    builder.print_gate_counts(0);

    log::debug!("before build recurisve {} circuit", N);
    let circuit_data = builder.build::<C>();
    log::debug!("after build recurisve {} circuit", N);

    (
        circuit_data,
        RecursiveTargets::<N> {
            size: N,
            proof_with_pub_input_targets: proof_with_pub_input_targets,
            verifier_circuit_target: verifier_circuit_target,
        },
    )
}
