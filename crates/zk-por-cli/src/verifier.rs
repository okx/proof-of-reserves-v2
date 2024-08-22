use indicatif::ProgressBar;
use rayon::iter::IntoParallelRefIterator;
use serde_json::from_reader;
use std::{fs::File, path::PathBuf};

// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use super::constant::{RECURSION_BRANCHOUT_NUM, USER_VERIFICATION_THREADS_NUM};
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
use rayon::iter::ParallelIterator;

use glob::glob;
use std::io;

fn find_matching_files(pattern: &str) -> Result<Vec<PathBuf>, io::Error> {
    let mut matching_files = Vec::new();

    // Use the glob function to get an iterator of matching paths
    for entry in glob(pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => matching_files.push(path),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }

    Ok(matching_files)
}

pub fn verify_user(
    global_proof_path: PathBuf,
    user_proof_path_pattern: &String,
) -> Result<(), PoRError> {
    let proof_file = File::open(&global_proof_path).unwrap();
    let reader = std::io::BufReader::new(proof_file);

    // Parse the JSON as Proof
    let proof: Proof = from_reader(reader)
        .expect(format!("fail to parse global proof from path {:?}", global_proof_path).as_str());

    let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
    let root_hash = HashOut::<F>::from_partial(&proof.proof.public_inputs[hash_offset]);
    let user_proof_paths =
        find_matching_files(user_proof_path_pattern).map_err(|e| PoRError::Io(e))?;
    let proof_file_num = user_proof_paths.len();
    println!("successfully identify {} user proof files", proof_file_num);

    let bar = ProgressBar::new(proof_file_num as u64);
    // user_proof_paths.chunk_by(pred)
    user_proof_paths.chunks(USER_VERIFICATION_THREADS_NUM).for_each(|chunks| {
        chunks.par_iter().for_each(|user_proof_path| {
            let merkle_path = File::open(&user_proof_path).unwrap();
            let reader = std::io::BufReader::new(merkle_path);
            let proof: MerkleProof = from_reader(reader).unwrap();
            if let Err(e) = proof.verify_merkle_proof(root_hash) {
                panic!(
                    "fail to verify the user proof on path {:?} due to error {:?}",
                    user_proof_path, e
                )
            }
        });
        bar.inc(chunks.len() as u64);
    });
    bar.finish();
    println!(
        "successfully verify {} user proofs with file pattern {}",
        proof_file_num, user_proof_path_pattern
    );

    Ok(())
}

pub fn verify_global(global_proof_path: PathBuf) -> Result<(), PoRError> {
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

    Ok(())
}
