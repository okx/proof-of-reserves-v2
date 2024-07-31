use plonky2::{
    fri::proof, hash::{
        hash_types::{HashOutTarget, MerkleCapTarget},
        merkle_proofs::MerkleProofTarget,
        poseidon::PoseidonHash,
    }, iop::target::Target, plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CommonCircuitData, VerifierCircuitTarget},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputsTarget,
    }
};

use crate::merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget;

use crate::types::{D, F};

pub struct RecursiveTargets<const N: usize> {
    pub size: usize,
    pub proof_with_pub_input_targets: Vec<ProofWithPublicInputsTarget<D>>,
    pub verifier_circuit_target: VerifierCircuitTarget,
    // pub vd_proof_target: VdProofTarget,
}

// pub struct VdProofTarget {
//     pub vd_digest_target: Vec<Target>,
//     pub vd_root_target: HashOutTarget,
//     pub vd_proof_target: MerkleProofTarget,
//     pub vd_index_target: Target,
// }

// pub fn build_vd_circuit(builder: &mut CircuitBuilder<F, D>, vd_proof_len: usize) -> VdProofTarget {
//     let vd_digest_target = builder.add_virtual_targets(68);
//     let vd_root_target = builder.add_virtual_hash();
//     let vd_proof_target = MerkleProofTarget {
//         siblings: builder.add_virtual_hashes(vd_proof_len),
//     };
//     let vd_index_target = builder.add_virtual_target();
//     let vd_index_bits = builder.split_le(vd_index_target, vd_proof_len);

//     // builder.verify_merkle_proof_to_cap::<PoseidonHash>(
//     //     vd_digest_target.clone(),
//     //     &vd_index_bits,
//     //     &MerkleCapTarget(vec![vd_root_target]),
//     //     &vd_proof_target,
//     // );

//     VdProofTarget {
//         vd_digest_target,
//         vd_root_target,
//         vd_proof_target,
//         vd_index_target,
//     }
// }

pub fn build_recursive_n_circuit<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    common_circuit_data: &CommonCircuitData<F, D>,
) -> RecursiveTargets<N>
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    let verifier_circuit_target = VerifierCircuitTarget {
        constants_sigmas_cap: builder
            .add_virtual_cap(common_circuit_data.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };

	let mut proof_with_pub_input_targets : Vec<ProofWithPublicInputsTarget<D>> = vec![];
	(0..N).for_each(|_| {
        let proof_with_pub_input_target =
            builder.add_virtual_proof_with_pis::<InnerC>(common_circuit_data);
        builder.verify_proof::<InnerC>(
            &proof_with_pub_input_target,
            &verifier_circuit_target,
            common_circuit_data,
        );
		proof_with_pub_input_targets.push(proof_with_pub_input_target);
	});

	let mut merkle_sum_node_targets : [MerkleSumNodeTarget;N] = [MerkleSumNodeTarget::default();N];
	merkle_sum_node_targets[0] = MerkleSumNodeTarget::from(proof_with_pub_input_targets[0].public_inputs.clone());
	(1..N).for_each(|i| {
		merkle_sum_node_targets[i] = MerkleSumNodeTarget::get_child_from_parents(builder, &merkle_sum_node_targets[i-1], &MerkleSumNodeTarget::from(proof_with_pub_input_targets[i].public_inputs.clone()));
	});
	merkle_sum_node_targets[N-1].registered_as_public_inputs(builder);

	// TODO: 
    // let vd_proof_target = build_vd_circuit(builder, vd_proof_len);
    // builder.register_public_inputs(vd_proof_target.vd_root_target.elements.as_slice()); // must be done after parent_node_target is registered as public inputs

    RecursiveTargets::<N> {
        size: N,
		proof_with_pub_input_targets: proof_with_pub_input_targets,
		verifier_circuit_target: verifier_circuit_target,	
    }
}
