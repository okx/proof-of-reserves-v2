use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::{CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData},
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputsTarget,
};

use crate::types::{D, F};

/// Targets for the verification of the subproofs in our recursive circuit during the recursive proving.
pub struct VerificationTargets {
    pub verifier_circuit_targets: VerifierCircuitTarget,
    pub proof_with_pis_targets: Vec<ProofWithPublicInputsTarget<D>>,
}

pub fn verify_n_subproof_circuit<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    subproof_common_cd: &CommonCircuitData<F, D>,
    subproof_verifier_cd: &VerifierOnlyCircuitData<InnerC, D>,
) -> VerificationTargets
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

    VerificationTargets { verifier_circuit_targets, proof_with_pis_targets }
}
