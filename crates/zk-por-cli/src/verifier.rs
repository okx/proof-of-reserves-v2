use serde_json::from_reader;
use std::{fs::File, path::PathBuf};

// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use super::constant::RECURSION_BRANCHOUT_NUM;
use zk_por_core::{
    circuit_config::{get_recursive_circuit_configs, STANDARD_CONFIG},
    circuit_registry::registry::CircuitRegistry,
    error::PoRError,
    merkle_proof::MerkleProof,
    recursive_prover::recursive_circuit::RecursiveTargets,
    types::F,
    Proof,
};

use plonky2::hash::hash_types::HashOut;

pub fn verify(
    global_proof_path: PathBuf,
    merkle_inclusion_path: Option<PathBuf>,
) -> Result<(), PoRError> {
    let proof_file = File::open(&global_proof_path).unwrap();
    let reader = std::io::BufReader::new(proof_file);

    // Parse the JSON as Proof
    let proof: Proof = from_reader(reader).unwrap();

    if proof.general.recursion_branchout_num != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }

    let token_num = proof.general.token_num;
    let batch_num = proof.general.batch_num;
    let round_num = proof.general.round_num;
    let batch_size = proof.general.batch_size;
    let recursive_circuit_configs =
        get_recursive_circuit_configs::<RECURSION_BRANCHOUT_NUM>(batch_num);

    // not to use trace::log to avoid the dependency on the trace config.
    println!(
        "start to reconstruct the circuit with {} recursive levels for round {}",
        recursive_circuit_configs.len(),
        round_num
    );
    let start = std::time::Instant::now();
    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

    let root_circuit = circuit_registry.get_root_circuit();

    let circuit_vd = root_circuit.verifier_only.circuit_digest;
    if circuit_vd != proof.root_vd_digest {
        return Err(PoRError::CircuitDigestMismatch);
    }

    println!(
        "successfully reconstruct the circuit for round {} in {:?}",
        round_num,
        start.elapsed()
    );
    let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
    let root_hash = HashOut::<F>::from_partial(&proof.proof.public_inputs[hash_offset]);
    if !root_circuit.verify(proof.proof).is_ok() {
        return Err(PoRError::InvalidProof);
    }
    println!("successfully verify the global proof for round {}", round_num);

    if let Some(merkle_inclusion_path) = merkle_inclusion_path {
        let merkle_path = File::open(&merkle_inclusion_path).unwrap();
        let reader = std::io::BufReader::new(merkle_path);

        // Parse the JSON as Proof
        let proof: MerkleProof = from_reader(reader).unwrap();

        let res = proof.verify_merkle_proof(root_hash);

        if res.is_err() {
            let res_err = res.unwrap_err();
            return Err(res_err);
        } else {
            println!("successfully verify the inclusion proof for user for round {}", round_num);
            return Ok(());
        }
    }

    Ok(())
}
