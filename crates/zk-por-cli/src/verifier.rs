use plonky2::fri::verifier;
use serde_json::from_reader;
use std::{fs::File, path::PathBuf};

// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use super::constant::RECURSION_BRANCHOUT_NUM;
use zk_por_core::{
    account::Account,
    circuit_config::{get_recursive_circuit_configs, STANDARD_CONFIG},
    circuit_registry::registry::CircuitRegistry,
    error::PoRError,
    global::GLOBAL_MST,
    merkle_proof::MerkleProof,
    Proof,
};

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

    if !root_circuit.verify(proof.proof).is_ok() {
        return Err(PoRError::InvalidProof);
    }
    println!("successfully verify the global proof for round {}", round_num);

    if let Some(merkle_inclusion_path) = merkle_inclusion_path {
        let merkle_path = File::open(&merkle_inclusion_path).unwrap();
        let reader = std::io::BufReader::new(merkle_path);

        // Parse the JSON as Proof
        let (proof, account): (MerkleProof, Account) = from_reader(reader).unwrap();

        let global_mst = GLOBAL_MST.get().unwrap();
        let mut _g = global_mst.read().expect("unable to get a lock");
        let gmst_root = _g.inner.last().unwrap();

        let merkle_inlcusion_proof_res =
            proof.verify_merkle_proof(&account, *gmst_root).expect("Invalid Merkle Proof");

        println!("successfully verify the inclusion proof for user for round {}", round_num);
    }

    Ok(())
}
