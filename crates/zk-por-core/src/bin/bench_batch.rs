use zk_por_core::{
    account::gen_accounts_with_random_data,
    merkle_sum_prover::{
        circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
    },
};

fn main() {
	let num_accounts = 1024; // configure this for bench.

	let num_assets = 5;
	let (circuit_data, account_targets) = build_merkle_sum_tree_circuit(num_accounts, num_assets);

	let (accounts, _, _) = gen_accounts_with_random_data(num_accounts, num_assets);
	let start = std::time::Instant::now();
	let prover = MerkleSumTreeProver { accounts };
	_ = prover.prove_with_circuit(&circuit_data, account_targets);

	println!("prove {} accounts in batch in : {:?}", num_accounts, start.elapsed());
}