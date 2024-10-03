use indicatif::ProgressBar;
use plonky2::{plonk::circuit_data::CircuitConfig, util::serialization::DefaultGateSerializer};
use plonky2_field::types::PrimeField64;
use rayon::iter::IntoParallelRefIterator;
use serde_json::from_reader;
use std::{fs::File, path::PathBuf};
// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use super::constant::RECURSION_BRANCHOUT_NUM;
use zk_por_core::{
    circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG},
    circuit_registry::{precompiled_registry::get_verifier_for_round, registry::CircuitRegistry},
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

// some proof files may not contain the circuit_configs field, we hardcode the default config in this case.
fn default_circuit_configs() -> (CircuitConfig, Vec<CircuitConfig>) {
    (STANDARD_CONFIG, vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG])
}

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
    verbose: bool,
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
    if proof_file_num == 0 {
        return Err(PoRError::InvalidParameter(format!(
            "fail to find any user proof files with pattern {}",
            user_proof_path_pattern
        )));
    }

    if verbose {
        println!("successfully identify {} user proof files", proof_file_num);
    }

    let bar = ProgressBar::new(proof_file_num as u64);
    let invalid_proof_paths = user_proof_paths
        .par_iter()
        .map(|user_proof_path| {
            let merkle_path = File::open(&user_proof_path).unwrap();
            let reader = std::io::BufReader::new(merkle_path);
            let proof: MerkleProof = from_reader(reader).expect(
                format!("fail to parse user proof from path {:?}", user_proof_path).as_str(),
            );
            let result = proof.verify_merkle_proof(root_hash);
            if verbose {
                bar.inc(1);
            }
            (result, user_proof_path)
        })
        .filter(|(result, _)| result.is_err())
        .map(|(_, invalid_proof_path)| invalid_proof_path.to_str().unwrap().to_owned())
        .collect::<Vec<String>>();
    if verbose {
        bar.finish();
    }

    let invalid_proof_num = invalid_proof_paths.len();
    let valid_proof_num = proof_file_num - invalid_proof_num;
    if verbose {
        let max_to_display_valid_proof_num = 10;

        println!(
            "{}/{} user proofs pass the verification. {} fail, the first {} failed proof files: {:?}",
            valid_proof_num, proof_file_num, invalid_proof_num, std::cmp::min(invalid_proof_num, invalid_proof_num), invalid_proof_paths.iter().take(max_to_display_valid_proof_num).collect::<Vec<&String>>(),
        );
    }

    if invalid_proof_num > 0 {
        return Err(PoRError::InvalidProof);
    }
    Ok(())
}

pub fn verify_global(
    global_proof_path: PathBuf,
    check_circuit: bool,
    verbose: bool,
) -> Result<(), PoRError> {
    let proof_file = File::open(&global_proof_path).unwrap();
    let reader = std::io::BufReader::new(proof_file);

    // Parse the JSON as Proof
    let proof: Proof = from_reader(reader).unwrap();

    if proof.general.recursion_branchout_num != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }
    let round_num = proof.general.round_num;
    let root_circuit_verifier_data = get_verifier_for_round(round_num);

    let round_num = proof.general.round_num;
    if check_circuit {
        let token_num = proof.general.token_num;
        let round_num = proof.general.round_num;
        let batch_size = proof.general.batch_size;

        // There are cases that the proof file does not contain the circuit_configs field, we hardcode the default config in this case.
        let (mut batch_circuit_config, mut recursive_circuit_configs) = default_circuit_configs();

        if let Some(circuit_configs) = &proof.circuit_configs {
            recursive_circuit_configs = circuit_configs.recursive_circuit_configs.clone();
            batch_circuit_config = circuit_configs.batch_circuit_config.clone();
        }

        if verbose {
            println!(
                "start to reconstruct the circuit with {} recursive levels for round {}",
                recursive_circuit_configs.len(),
                round_num
            );
        }
        let start = std::time::Instant::now();
        let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
            batch_size,
            token_num,
            batch_circuit_config,
            recursive_circuit_configs,
        );

        let rebuilt_root_circuit_verifier_data =
            circuit_registry.get_root_circuit().verifier_data();

        if rebuilt_root_circuit_verifier_data.verifier_only.circuit_digest
            != root_circuit_verifier_data.verifier_only.circuit_digest
        {
            return Err(PoRError::CircuitMismatch);
        }
        if verbose {
            println!(
                "successfully reconstruct the circuit for round {} in {:?}",
                round_num,
                start.elapsed()
            );
        }
    }
    if proof.root_vd_digest != root_circuit_verifier_data.verifier_only.circuit_digest {
        return Err(PoRError::CircuitMismatch);
    }

    let result = root_circuit_verifier_data.verify(proof.proof.clone());

    if verbose {
        let equity = proof.proof.public_inputs
            [RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_equity_offset()];
        let debt = proof.proof.public_inputs
            [RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_debt_offset()];
        if result.is_ok() {
            println!("successfully verify the global proof for round {}, total exchange users' equity is {}, debt is {}, exchange liability is {}",
            round_num, equity.to_canonical_u64(), debt.to_canonical_u64(), (equity - debt).to_canonical_u64());
        } else {
            println!("fail to verify the global proof for round {}, total exchange users' equity is {}, debt is {}, exchange liability is {}",
            round_num, equity.to_canonical_u64(), debt.to_canonical_u64(), (equity - debt).to_canonical_u64());
        }
    }

    result.map_err(|_| PoRError::InvalidProof)
}

pub fn print_circuit_verifier_hex(global_proof_path: PathBuf) -> Result<(), PoRError> {
    let proof_file = File::open(&global_proof_path).unwrap();
    let reader = std::io::BufReader::new(proof_file);
    let proof: Proof = from_reader(reader).unwrap();

    if proof.general.recursion_branchout_num != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }
    let token_num = proof.general.token_num;
    let batch_size = proof.general.batch_size;

    let (mut batch_circuit_config, mut recursive_circuit_configs) = default_circuit_configs();

    if let Some(circuit_configs) = &proof.circuit_configs {
        recursive_circuit_configs = circuit_configs.recursive_circuit_configs.clone();
        batch_circuit_config = circuit_configs.batch_circuit_config.clone();
    }

    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        batch_circuit_config,
        recursive_circuit_configs,
    );

    let root_circuit = circuit_registry.get_root_circuit();

    let verifier_data = root_circuit.verifier_data().clone();

    let gate_serializer = DefaultGateSerializer;
    let verifier_data_bytes = verifier_data.to_bytes(&gate_serializer).unwrap();

    if proof.root_vd_digest != verifier_data.verifier_only.circuit_digest {
        return Err(PoRError::CircuitMismatch);
    }

    println!("{}", hex::encode(verifier_data_bytes));

    Ok(())
}
