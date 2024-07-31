use log::Level;
use plonky2::{
    field::extension::Extendable, fri::proof, gates::noop::NoopGate, hash::{
        hash_types::{HashOut, RichField},
        poseidon_bn128::PoseidonBN128Hash,
    }, iop::witness::{PartialWitness, WitnessWrite}, plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{
            CircuitConfig, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
        },
        config::{AlgebraicHasher, GenericConfig, GenericHashOut, Hasher},
        proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget},
        prover::prove,
    }, util::timing::TimingTree
};

use super::super::circuits::circuit_config;
use crate::types::{C, D, F};
use anyhow::Result;
use lazy_static::lazy_static;
use serde_json;
use std::{fs, path::PathBuf};

use super::circuit;
use super::vd::{VDProof, VdMap};

pub type ProofTuple<F, C, const D: usize> = (
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, D>,
);

// pub fn set_recursive_targets<C: GenericConfig<D, F = F>, InnerC: GenericConfig<D, F = F>>(
//     pw: &mut PartialWitness<F>,
//     target: &circuit::RecursiveTarget,
//     pi: &ProofWithPublicInputs<F, InnerC, D>,
//     vd: &VerifierOnlyCircuitData<InnerC, D>,
//     vd_proof: &VDProof<F, D>,
// ) where
//     InnerC::Hasher: AlgebraicHasher<F>,
//     // [(); C::Hasher::HASH_SIZE]:,
// {
//     pw.set_proof_with_pis_target(&target.proof_with_pub_input_target, pi);

//     pw.set_hash_target(target.vd_proof_target.vd_root_target, vd_proof.root);
//     pw.set_target(target.vd_proof_target.vd_index_target, vd_proof.index);

//     for (vd_target, vd_element) in target.vd_proof_target.vd_digest_target.iter().zip(
//         [
//             vd.constants_sigmas_cap.flatten().as_slice(),
//             vd.circuit_digest.to_vec().as_slice(),
//         ]
//         .concat(),
//     ) {
//         pw.set_target(vd_target.clone(), vd_element);
//     }
//     for i in 0..vd_proof.merkle_proof.siblings.len() {
//         pw.set_hash_target(
//             target.vd_proof_target.vd_proof_target.siblings[i],
//             vd_proof.merkle_proof.siblings[i],
//         );
//     }
// }

// fn prove_16_subproofs<C: GenericConfig<D, F = F>, InnerC: GenericConfig<D, F = F>>(
//     left_subproof: &ProofTuple<F, InnerC, D>,
//     right_subproof: &ProofTuple<F, InnerC, D>,
//     left_vd_proof: &VDProof<F, D>,
//     right_vd_proof: &VDProof<F, D>,
//     config: &CircuitConfig,
// ) -> Result<(
//     ProofWithPublicInputs<F, C, D>,
//     VerifierOnlyCircuitData<C, D>,
//     CommonCircuitData<F, D>,
// )>
// where
//     InnerC::Hasher: AlgebraicHasher<F>,
//     // [(); C::Hasher::HASH_SIZE]:, // TODO: figure out how to make this work
// {
//     let (left_proof_with_pub_input, left_vd, left_cd) = left_subproof;
//     let (right_proof_with_pub_input, right_vd, right_cd) = right_subproof;

//     let mut builder = CircuitBuilder::<F, D>::new(config.clone());

//     // Builds the recursive circuit for checking vd_proof and constraints
//     let vd_proof_len = left_vd_proof.merkle_proof.siblings.len();
//     println!("vd_proof_len: {:?}", vd_proof_len);
//     let recursive_targets =
//         circuit::build_recursive_circuit::<InnerC>(&mut builder, left_cd, right_cd, vd_proof_len);
//     let circuit::RecursiveTargets { targets } = recursive_targets;

//     let mut pw = PartialWitness::new();

//     set_recursive_targets::<C, InnerC>(
//         &mut pw,
//         &targets[0],
//         left_proof_with_pub_input,
//         left_vd,
//         left_vd_proof,
//     );

//     set_recursive_targets::<C, InnerC>(
//         &mut pw,
//         &targets[1],
//         right_proof_with_pub_input,
//         right_vd,
//         right_vd_proof,
//     );

//     #[cfg(debug_assertions)]
//     builder.print_gate_counts(0);

//     let data = builder.build::<C>();
//     println!("after build");
//     let mut timing = TimingTree::new("prove_two_subproofs", log::Level::Debug);
//     let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;

//     #[cfg(debug_assertions)]
//     data.verify(proof.clone())?;

//     Ok((proof, data.verifier_only, data.common))
// }

pub fn prove_n_subproofs<C: GenericConfig<D, F = F>, InnerC: GenericConfig<D, F = F>, const N: usize>(
    proofs: [ProofTuple<F, InnerC, D>;N],
    config: &CircuitConfig,
) -> Result<(
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, D>,
)>
where
    InnerC::Hasher: AlgebraicHasher<F>,
    // [(); C::Hasher::HASH_SIZE]:, // TODO: figure out how to make this work
{
    // get circuit data only from the first proof, as the subsequent proofs are constrained to be identical to the first proof. 
    let (_, verifier_circuit_data, common_circuit_data) = &proofs[0];

    let mut builder = CircuitBuilder::<F, D>::new(config.clone());

    let recursive_targets =
        circuit::build_recursive_n_circuit::<InnerC, N>(&mut builder, &common_circuit_data);

    let mut pw = PartialWitness::new();
    pw.set_verifier_data_target(&recursive_targets.verifier_circuit_target, &verifier_circuit_data);

    (0..N).for_each(|i| {
        let (proof_with_pub_input, _ , _) = &proofs[i];
        pw.set_proof_with_pis_target(&recursive_targets.proof_with_pub_input_targets[i], proof_with_pub_input);
    });

    #[cfg(debug_assertions)]
    builder.print_gate_counts(0);

    log::debug!("before build");
    let data = builder.build::<C>();
    log::debug!("after build");
    let mut timing = TimingTree::new("prove_N_subproofs", log::Level::Debug);
    let start = std::time::Instant::now();
    log::debug!("before prove");
    let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;

    #[cfg(debug_assertions)] {
        log::debug!("time for {:?} proofs, {:?}", N, start.elapsed().as_millis());
        data.verify(proof.clone())?;
    }

    Ok((proof, data.verifier_only, data.common))
}

// pub fn aggregate_proofs_at_level<C: GenericConfig<D, F = F>, InnerC: GenericConfig<D, F = F>, const N: usize>(
//     config: &CircuitConfig,
//     proofs: &Vec<ProofTuple<F, InnerC, D>>,
//     vd_proofs: &Vec<VDProof<F, D>>,
//     level: usize,
// ) -> Result<()>
// where
//     InnerC::Hasher: AlgebraicHasher<F>,
//     // [(); C::Hasher::HASH_SIZE]:,
// {

//     assert_eq!(proofs.len(), N);
//     if proofs.len() != vd_proofs.len() {
//         return Err(anyhow::anyhow!(format!(
//             "number of proofs [{}] is not consistent with number of vd_proofs [{}]",
//             proofs.len(),
//             vd_proofs.len()
//         )));
//     }

//     if proofs.len() == 0 {
//         return Err(anyhow::anyhow!("no proofs to aggregate"));
//     }

//     if proofs.len() % 2 == 1 {
//         return Err(anyhow::anyhow!(format!(
//             "number of proofs [{}] is not even",
//             proofs.len()
//         )));
//     }

//     log::debug!(
//         "start to aggregate {:?} proofs on level {:?} ",
//         proofs.len(),
//         level
//     );
//     let now = std::time::Instant::now();


//     // let recursive_proofs = proofs
//     //     .iter()
//     //     .enumerate()
//     //     .filter(|(i, _)| *i < proofs.len() / 2)
//     //     .map(|(i, _)| {
//     //         let res = 
//     //         res.unwrap()
//     //     })
//     //     .collect();
//     let _ = prove_n_subproofs::<C, InnerC, N>(
//         &proofs,
//         config,
//     );
//     log::debug!(
//         "finish aggregating {:?} proofs on level {:?} in {:?}",
//         proofs.len(),
//         level,
//         now.elapsed().as_millis()
//     );
//     Ok(())
// }

// pub fn batch_prove_recursive_proofs(
//     leaf_proofs: Vec<ProofTuple<F, C, D>>,
// ) -> Result<ProofTuple<F, C, D>> {
//     let mut level: usize = 0;
//     let mut proofs = leaf_proofs;
//     while proofs.len() > 1 {
//         let vd_proofs = proofs
//             .iter()
//             .map(|proof| VD_MAP.get(&proof.1.digest()).unwrap().clone())
//             .collect::<Vec<VDProof<F, D>>>();

//         proofs = aggregate_proofs_at_level::<C, C>(
//             &circuit_config::STANDARD_CONFIG,
//             &proofs,
//             &vd_proofs,
//             level,
//         )
//         .unwrap();
//         level += 1;
//     }
//     Ok(proofs[0].clone())
// }

// lazy_static! {
//     pub static ref VD_MAP: VdMap<F, D> = {
//         let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
//         let path = PathBuf::from(manifest_dir).join("static/vd_map");
//         let loaded_doc = fs::read_to_string(path).unwrap();
//         let vd_map = serde_json::from_str::<VdMap<F, D>>(&loaded_doc).unwrap();
//         vd_map
//     };
// }

