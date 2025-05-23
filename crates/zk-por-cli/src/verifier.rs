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
    circuit_registry::registry::CircuitRegistry,
    error::PoRError,
    merkle_proof::MerkleProof,
    recursive_prover::recursive_circuit::RecursiveTargets,
    types::{C, D, F},
    Proof,
};

use plonky2::{hash::hash_types::HashOut, plonk::circuit_data::VerifierCircuitData};
use rayon::iter::ParallelIterator;

use glob::glob;
use std::io;

static ROOT_CIRCUIT_HEX_FOR_508787475: &str = "0100000000000000e3370d1646daebec7fa045ddf1cc918706777cb88875ea173623eff57b773bc68f62cdf279612bd8c095eb7bad5feaf5209df02981e0b88ef5b178862e00694ba296ce1215562eabcac32a3b9a5aae1dcf6422a739a392ffc1b87f102bdf523d8700000000000000500000000000000002000000000000006400000000000000020000000000000008000000000000000101030000000000000001000000000000001c00000000000000100000000104000000000000000500000000000000030000000000000001000000000000001c00000000000000100000000104000000000000000500000000000000040000000000000004000000000000000400000000000000040000000000000004000000000000001300000000000000010e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000001000000000000000100000000000000010000000000000001000000000000000200000000000000020000000000000003000000000000000000000000000000070000000000000007000000000000000c000000000000000c000000000000000e0000000000000008000000000000007b0000000000000005000000000000000600000000000000500000000000000001000000000000000700000000000000310000000000000057010000000000006109000000000000a74100000000000091cb010000000000f7900c0000000000c1f657000000000047bf670200000000f13ad61000000000979cdb75000000002148013903000000e7f8088f1600000051ce3ee99d00000037a4b76051040000817d05a5391e0000876e268393d30000b1050d9608c90500d7275b1a3c7f2800e1167eb8a47a1b0127a0720b815ac007116122508779423676a7f030b452d17b37949456f042b9627f0d105e94d410b3755e709212d075e52d95120188b038463a148207b9d38ceb908d8e3415cad970eddee56f9786f4157b18490f24aeaf9959abff6a00c3cd336eaffdec0355a06a00ccef7a1d5362eafa938e5cd445b068d40be687d0e8d1dcc6524ab7b95dbd096a43080314902d44e5d739158df03edd3de79494e193b80cab5212102b0b0c59ab4280702f4e546faad281134f234e0ba6c28c8829f7224f8852d9bb24c2f429b741f122024fb12500cc98f40f29d90700942db06f1ff036ff0b3fd10edc9080f653b9b86a04f683b74b110dee1eba9bfd11795b86d81642ea7d4f80adeb9fce61712c82c3715fa6a319378f5c1c9c8c72b381ea8ac644d819e88b69d16de1e9a958d3e2bf002a659d6cc733410526c446f8736acd240a5de8c92be99f01478b55853260620bf1ce4ea561a1b54f97a81e85ab69fb2d239ccea3b0e3e341f644a17ad4393ccdbbe2615acf94ab9c2233a678ab11b248f265884cb07be0fc9fc9b317d26128ea5f83e2a5beac1d679f972a8936b9d3d15b2525c07d10cbbc8205034170738d29932614c71128df22060e8c717c181af42a62d21a67abb8ac2cafbabbd1af10b938ca1122bcce790f8d8709000000000000000e00000000000000090000000b0000000c000000020000003f000000000000000400000004000000000000000e0000000100000000000000140000000000000000000000000000000f0000002000000000000000100000002b00000000000000010000000a00000000000000000000001400000000000000080000000d000000000000000500000042000000000000000e0000000400000000000000040000000000000002000000000000000d000000";

// The proof file at round 508787475 on Sept 16th 2024 does not contain the circuit_infos, we hardcode in this case.
fn circuit_configs_for_508787475() -> (CircuitConfig, Vec<CircuitConfig>) {
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
    let proof_file = File::open(&global_proof_path).map_err(|e| {
        PoRError::InvalidParameter(format!(
            "fail to open {:?} due to error {:?}",
            global_proof_path, e
        ))
    })?;
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
    let proof_file = File::open(&global_proof_path).map_err(|e| {
        PoRError::InvalidParameter(format!(
            "fail to open {:?} due to error {:?}",
            global_proof_path, e
        ))
    })?;
    let reader = std::io::BufReader::new(proof_file);

    let proof: Proof = from_reader(reader).map_err(|_| PoRError::InvalidProof)?;

    if proof.general.recursion_branchout_num != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }
    let round_num = proof.general.round_num;
    let root_verifier_data_hex: String;
    let (mut batch_circuit_config, mut recursive_circuit_configs) =
        (CircuitConfig::default(), vec![]);
    (_, _) = (batch_circuit_config, recursive_circuit_configs); // To quell the compiler, i.e., circuit configs are useless if not rebuilding the circuit.

    if let Some(circuits_info) = proof.circuits_info {
        (batch_circuit_config, recursive_circuit_configs) =
            (circuits_info.batch_circuit_config, circuits_info.recursive_circuit_configs);
        root_verifier_data_hex = circuits_info.root_verifier_data_hex;
        // only round number 508787475 does not contain the circuit_infos field, we hardcode in this case.
    } else if round_num == 508787475 {
        (batch_circuit_config, recursive_circuit_configs) = circuit_configs_for_508787475();
        root_verifier_data_hex = ROOT_CIRCUIT_HEX_FOR_508787475.to_string();
    } else {
        return Err(PoRError::InvalidProof);
    }

    let root_circuit_verifier_data_bytes = hex::decode(root_verifier_data_hex)
        .expect("fail to decode root circuit verifier data hex string");

    let root_circuit_verifier_data = VerifierCircuitData::<F, C, D>::from_bytes(
        root_circuit_verifier_data_bytes,
        &DefaultGateSerializer,
    )
    .expect("fail to parse root circuit verifier data");

    let round_num = proof.general.round_num;
    if check_circuit {
        let token_num = proof.general.token_num;
        let round_num = proof.general.round_num;
        let batch_size = proof.general.batch_size;

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
