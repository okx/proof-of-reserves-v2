use indicatif::ProgressBar;
use plonky2_field::types::PrimeField64;
use rayon::iter::IntoParallelRefIterator;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{fs::File, path::PathBuf};
// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use super::constant::RECURSION_BRANCHOUT_NUM;
use std::io::Read;
use zk_por_core::{
    error::PoRError,
    merkle_proof::MerkleProof,
    recursive_prover::recursive_circuit::RecursiveTargets,
    types::{C, D, F},
    Proof,
};

use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::{CommonCircuitData, VerifierOnlyCircuitData},
        verifier::verify,
    },
    util::serialization::DefaultGateSerializer,
};
use rayon::iter::ParallelIterator;

use glob::glob;
use std::io;

#[derive(Serialize, Deserialize, Debug)]
struct VerifierData {
    circuit_common: String,
    verifier_only_data: String,
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
    vd_path: PathBuf,
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

    if verbose {
        let equity = proof.proof.public_inputs
            [RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_equity_offset()];
        let debt = proof.proof.public_inputs
            [RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_debt_offset()];

        let mut file = File::open(vd_path).unwrap();

        let mut json_string = String::new();
        file.read_to_string(&mut json_string).unwrap();
        let vd_deserialized: VerifierData = serde_json::from_str(&json_string).unwrap();

        let vd_recoverred: VerifierOnlyCircuitData<C, D> =
            serde_json::from_str(&vd_deserialized.verifier_only_data).unwrap();

        let gate_serializer = DefaultGateSerializer;
        let common_data_recoverred = CommonCircuitData::<F, D>::from_bytes(
            hex::decode(vd_deserialized.circuit_common).unwrap(),
            &gate_serializer,
        )
        .unwrap();

        let ret = verify(proof.proof, &vd_recoverred, &common_data_recoverred);

        if !ret.is_ok() {
            return Err(PoRError::InvalidProof);
        }

        println!("successfully verify the global proof for round {}, total exchange users' equity is {}, debt is {}, exchange liability is {}",
        round_num, equity.to_canonical_u64(), debt.to_canonical_u64(), (equity- debt).to_canonical_u64());
    }

    Ok(())
}
