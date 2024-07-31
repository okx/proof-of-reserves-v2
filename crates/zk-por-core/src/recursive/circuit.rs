use plonky2::{
    hash::{
        hash_types::{HashOutTarget, MerkleCapTarget},
        merkle_proofs::MerkleProofTarget,
        poseidon::PoseidonHash,
    },
    iop::target::Target,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CommonCircuitData, VerifierCircuitTarget},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputsTarget,
    },
};

use crate::circuits::merkle_sum_circuit::MerkleSumNodeTarget;

use crate::types::{D, F};

pub struct RecursiveTargets<const N: usize> {
    pub size: usize,
    pub targets: Vec<RecursiveTarget>,
    pub verifier_circuit_target: VerifierCircuitTarget,
}

pub struct RecursiveTarget {
    pub proof_with_pub_input_target: ProofWithPublicInputsTarget<D>,
    // pub verifier_circuit_target: VerifierCircuitTarget,
    // pub vd_proof_target: VdProofTarget,
}

pub struct VdProofTarget {
    pub vd_digest_target: Vec<Target>,
    pub vd_root_target: HashOutTarget,
    pub vd_proof_target: MerkleProofTarget,
    pub vd_index_target: Target,
}

pub fn build_vd_circuit(builder: &mut CircuitBuilder<F, D>, vd_proof_len: usize) -> VdProofTarget {
    let vd_digest_target = builder.add_virtual_targets(68);
    let vd_root_target = builder.add_virtual_hash();
    let vd_proof_target = MerkleProofTarget {
        siblings: builder.add_virtual_hashes(vd_proof_len),
    };
    let vd_index_target = builder.add_virtual_target();
    let vd_index_bits = builder.split_le(vd_index_target, vd_proof_len);

    // builder.verify_merkle_proof_to_cap::<PoseidonHash>(
    //     vd_digest_target.clone(),
    //     &vd_index_bits,
    //     &MerkleCapTarget(vec![vd_root_target]),
    //     &vd_proof_target,
    // );

    VdProofTarget {
        vd_digest_target,
        vd_root_target,
        vd_proof_target,
        vd_index_target,
    }
}

pub fn build_recursive_n_circuit<
    // C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    common_circuit_data: &CommonCircuitData<F, D>,
    // common_circuit_data1: &CommonCircuitData<F, D>,
    // num_of_proofs: u32,
    // vd_proof_len: usize,
) -> RecursiveTargets<N>
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    // let mut targets = [RecursiveTarget::default(); N];
    let verifier_circuit_target = VerifierCircuitTarget {
        constants_sigmas_cap: builder
            .add_virtual_cap(common_circuit_data.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };

    let ret = (0..N).map(|x| {
        let proof_with_pub_input_target =
            builder.add_virtual_proof_with_pis::<InnerC>(common_circuit_data);


        // let vd_proof_target_0 = build_vd_circuit(builder, vd_proof_len);
        // let right_node_target1 = MerkleSumNodeTarget::from(proof_with_pub_input_target1.public_inputs.clone());
        builder.verify_proof::<InnerC>(
            &proof_with_pub_input_target,
            &verifier_circuit_target,
            common_circuit_data,
        );

        RecursiveTarget {
            proof_with_pub_input_target: proof_with_pub_input_target,
            // vd_proof_target: vd_proof_target_0,
        }
    }).collect::<Vec<RecursiveTarget>>();

    // let proof_with_pub_input_target1 = builder.add_virtual_proof_with_pis::<InnerC>(common_circuit_data1);

    // let verifier_circuit_target1 = VerifierCircuitTarget {
    // 	constants_sigmas_cap: builder.add_virtual_cap(common_circuit_data1.config.fri_config.cap_height),
    // 	circuit_digest: builder.add_virtual_hash(),
    // };

    // let vd_proof_target_1 = build_vd_circuit(builder, vd_proof_len);
    // vd root should be equals for both vd proofs
    // builder.connect_hashes(
    // 	vd_proof_target_0.vd_root_target,
    // 	vd_proof_target_1.vd_root_target,
    // );

    // let parent_node_target = MerkleSumNodeTarget::get_child_from_parents(builder, &left_node_target0, &right_node_target1);

    // builder.verify_proof::<InnerC>(&proof_with_pub_input_target1, &verifier_circuit_target1, common_circuit_data0);

    // parent_node_target.registered_as_public_inputs(builder);
    // builder.register_public_inputs(vd_proof_target_0.vd_root_target.elements.as_slice()); // must be done after parent_node_target is registered as public inputs

    RecursiveTargets::<N> {
        size: N,
        targets: ret,
        verifier_circuit_target
    }
}
