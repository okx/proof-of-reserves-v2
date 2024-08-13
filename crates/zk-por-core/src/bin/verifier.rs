use std::env;
use std::fs::File;
use serde_json::from_reader;

// Assuming Proof is defined in lib.rs and lib.rs is in the same crate
use zk_por_core::{
	Proof, 
    circuit_config::{get_recursive_circuit_configs, STANDARD_CONFIG},
    circuit_registry::registry::CircuitRegistry,
};

/// cargo run --release --package zk-por-core --bin verifier --features verifier -- example_proof.json
fn main() {
    // Get the first argument as the JSON file path
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_json>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];

    // Open the file
    let file = File::open(file_path).unwrap();
    let reader = std::io::BufReader::new(file);

    // Parse the JSON as Proof
    let proof: Proof = from_reader(reader).unwrap();

    // Use the proof for something
    const RECURSION_BRANCHOUT_NUM: usize = 64;

    if proof.general.recursion_branchout_num != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }

    let asset_num = proof.general.asset_num;
	let batch_num = proof.general.batch_num;
	let round_num = proof.general.round_num;
	let batch_size = proof.general.batch_size;
	let recursive_circuit_configs = get_recursive_circuit_configs::<RECURSION_BRANCHOUT_NUM>(batch_num);

	// not to use trace::log to avoid the dependency on the trace config. 
	println!("start to reconstruct the circuit with {} recursive levels for round {}", recursive_circuit_configs.len(), round_num);
	let start = std::time::Instant::now();
    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

	let root_circuit = circuit_registry.get_root_circuit();
	assert_eq!(root_circuit.verifier_only.circuit_digest, proof.root_vd_digest);
	println!("successfully reconstruct the circuit for round {} in {:?}", round_num, start.elapsed());

	assert!(root_circuit.verify(proof.proof).is_ok());
	println!("successfully verify the proof for round {}", round_num);
}