use zk_por_core::merkle_sum_prover::{
    circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
};

use zk_por_core::{
    account::gen_accounts_with_random_data,
    recursive::{circuit::build_recursive_n_circuit, prove::prove_n_subproofs},
    types::C,
};

fn main() {
	const SUBPROOF_NUM : usize = 128; // configure this for bench. 

	let batch_size = 1024;
	let asset_num = 4;
	let (merkle_sum_circuit, account_targets) =
		build_merkle_sum_tree_circuit(batch_size, asset_num);
	println!("build merkle sum tree circuit");

	let accounts = gen_accounts_with_random_data(batch_size, asset_num).0;
	let prover = MerkleSumTreeProver { accounts };

	let merkle_sum_proof = prover.prove_with_circuit(&merkle_sum_circuit, account_targets).unwrap();
	println!("prove merkle sum tree");

	let (recursive_circuit, recursive_account_targets) = build_recursive_n_circuit::<C, SUBPROOF_NUM>(
		&merkle_sum_circuit.common,
		&merkle_sum_circuit.verifier_only,
	);
	println!("build recursive circuit");

	let mut subproofs = Vec::new();
	(0..SUBPROOF_NUM).for_each(|_| {
		subproofs.push(merkle_sum_proof.clone());
	});
	let start = std::time::Instant::now();
	_ = prove_n_subproofs(
		subproofs.clone(),
		&merkle_sum_circuit.verifier_only,
		&recursive_circuit,
		recursive_account_targets.clone(),
	);
	println!("prove recursive {} subproofs in : {:?}", SUBPROOF_NUM, start.elapsed());
}