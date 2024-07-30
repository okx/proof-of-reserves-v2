use std::env;

use log::{info, Level};
use plonky2::{iop::witness::PartialWitness, plonk::{circuit_builder::CircuitBuilder, circuit_data::{CircuitData, VerifierOnlyCircuitData}, prover::prove}, util::timing::TimingTree};
use plonky2_field::goldilocks_field::GoldilocksField;
use zk_por_core::{circuits::{account_circuit::{AccountSumTargets, AccountTargets}, circuit_config::STANDARD_CONFIG, merkle_sum_circuit::build_merkle_sum_tree_from_account_targets}, core::parser::read_json_into_accounts_vec, recursive::{vd::{VerifierDataDigest, VdTree}, circuit::build_recursive_circuit}, types::{C, D, F}};
use std::time::Instant;
use std::{fs::File, io::Write, path::PathBuf};

fn get_batch_circuit_vd(batch_size: usize, asset_count : usize) -> CircuitData<F, C, D> {
	let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    // let mut pw = PartialWitness::<GoldilocksField>::new();

    let mut account_targets: Vec<AccountTargets> = Vec::new();

    for _ in 0..batch_size {
        let asset_targets = builder.add_virtual_targets(asset_count);
        let debt_targets = builder.add_virtual_targets(asset_count);
        let account_target = AccountTargets{
            assets: asset_targets,
            debt: debt_targets,
        };

        // account_target.set_account_targets(accounts.get(i).unwrap(), &mut pw);
        account_targets.push(
            account_target
        );
    }

    let mut account_sum_targets: Vec<AccountSumTargets> = account_targets.iter().map(|x| AccountSumTargets::from_account_target(x, &mut builder)).collect();
    let _merkle_tree_targets = build_merkle_sum_tree_from_account_targets(&mut builder, &mut account_sum_targets);

    builder.print_gate_counts(0);
	let start = Instant::now();
    let data = builder.build::<C>();
	let duration = start.elapsed();

// Output the duration in milliseconds
	log::info!("Build Batch Circuit Duration: {} ms", duration.as_millis());
	data
}

fn get_recursive_circuit_data(child_circuit_data : CircuitData<F, C, D>, recursive_depth : usize) -> CircuitData<F,C,D> {
	let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);

	build_recursive_circuit::<C>(&mut builder, &child_circuit_data.common, &child_circuit_data.common, recursive_depth);
	let data = builder.build::<C>();
	data
}

fn next_power_of_two(n: usize) -> usize {
	let mut k = 1;
	while k < n {
		k <<= 1;
	}
	k
}

fn main() {
	env_logger::init();
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: <program> <batch_size> <asset_count> <recursive_depth>");
        std::process::exit(1);
    }

    let batch_size: usize = args[1].parse().expect("batch_size must be an unsigned integer");
    let asset_count: usize = args[2].parse().expect("asset_count must be an unsigned integer");
    let recursive_depth: usize = args[3].parse().expect("recursive_depth must be an unsigned integer");

	let batch_circuit = get_batch_circuit_vd(batch_size, asset_count);
	log::info!("batch circuit vd digest: {:#?}", batch_circuit.verifier_only.digest());

	let mut last_level_circuit = batch_circuit;

	let mut vd_tree: VdTree<F, D> = VdTree::new(
		vec![last_level_circuit.verifier_only.digest()],
		next_power_of_two(1 + recursive_depth),
    );
	let mut last_vd_digest = last_level_circuit.verifier_only.digest();
	for level in (0..recursive_depth).rev() {
		log::info!("Building recursive circuit at level {}", level);
		let start = Instant::now();
		last_level_circuit = get_recursive_circuit_data(last_level_circuit, 2);
		let duration = start.elapsed();
		log::info!("recursive vd digest: {:#?} at level {}, building in {} ms. ", last_level_circuit.verifier_only.digest(), level, duration.as_millis());

		if last_level_circuit.verifier_only.digest().iter().zip(last_vd_digest.iter()).all(|(&a, &b)| {
			a.eq(&b)
		}) {
			log::info!("early terminate at level {}", level);
			break;
		}
		last_vd_digest = last_level_circuit.verifier_only.digest();

		vd_tree.update_vd_digests::<C>(vec![last_level_circuit.verifier_only.digest()]);
	}

	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let path = PathBuf::from(manifest_dir).join(format!("static/vd_map_batch_size_{}_asset_count_{}_recursive_depth_{}.json", batch_size, asset_count, recursive_depth));
	let path1 = path.clone();
    let mut vd_file = File::create(path).unwrap();
    vd_file.write_all(serde_json::to_string(&vd_tree.vd_proof_map).unwrap().as_ref()).unwrap();
	log::info!("finish dumping vd map to file to {:?}", path1);
}