use criterion::{black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion, measurement::WallTime, SamplingMode};
use zk_por_core::{
	circuit_config::STANDARD_CONFIG,
	account::gen_accounts_with_random_data,
	recursive_prover::prover::RecursiveProver,
	recursive_prover::recursive_circuit::build_recursive_n_circuit,
	merkle_sum_prover::prover::MerkleSumTreeProver,
	merkle_sum_prover::circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit,
	types::{C, D, F},
};

use plonky2::plonk::proof::ProofWithPublicInputs;

pub fn bench_batch_circuit(c: &mut BenchmarkGroup<WallTime>, batch_size: usize) {
	let num_assets = 5;
    let (circuit_data, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, num_assets, STANDARD_CONFIG);
    let accounts = gen_accounts_with_random_data(batch_size, num_assets);

    c.bench_function(format!("batch_circuit_{}", batch_size).as_str(), |b| b.iter( || {
		let prover = MerkleSumTreeProver { accounts: accounts.clone() };
		_ = prover.get_proof_with_circuit_data(account_targets.clone(), &circuit_data);
	}));
}

pub fn batch_circuit(c : &mut Criterion) {
	let mut group = c.benchmark_group("batch_circuit");
	group.sample_size(10);
	bench_batch_circuit(&mut group, 16);
	bench_batch_circuit(&mut group, 64);
	bench_batch_circuit(&mut group, 256);
	bench_batch_circuit(&mut group, 512);
	bench_batch_circuit(&mut group, 1024);
	group.finish();
}

pub fn bench_recursive_circuit<const SUBPROOF_NUM : usize>(g: &mut BenchmarkGroup<WallTime>) {
    let batch_size = 1024;
    let asset_num = 4;
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num, STANDARD_CONFIG);

    let accounts = gen_accounts_with_random_data(batch_size, asset_num);
    let prover = MerkleSumTreeProver { accounts };

    let merkle_sum_proof = prover.get_proof_with_circuit_data(account_targets, &merkle_sum_circuit);

    let (recursive_circuit, recursive_targets) = build_recursive_n_circuit::<C, SUBPROOF_NUM>(
        &merkle_sum_circuit.common,
        &merkle_sum_circuit.verifier_only,
        STANDARD_CONFIG,
    );

    let subproofs: [ProofWithPublicInputs<F, C, D>; SUBPROOF_NUM] =
        std::array::from_fn(|_| merkle_sum_proof.clone());

	g.bench_function(format!("recursive_circuit_{}", SUBPROOF_NUM).as_str(), |b| b.iter( || {
		let recursive_prover = RecursiveProver {
			sub_proofs: subproofs.clone(),
			sub_circuit_vd: merkle_sum_circuit.verifier_only.clone(),
		};
		recursive_prover.get_proof_with_circuit_data(recursive_targets.clone(), &recursive_circuit);
	}));
}

pub fn recursive_circuit(c : &mut Criterion) {
	let mut group = c.benchmark_group("recursive_circuit");
	group.sample_size(10);
	group.sampling_mode(SamplingMode::Flat); // for long running benchmarks
	bench_recursive_circuit::<4>(&mut group);
	bench_recursive_circuit::<8>(&mut group);
	bench_recursive_circuit::<16>(&mut group);
	bench_recursive_circuit::<32>(&mut group);
	bench_recursive_circuit::<64>(&mut group);
	group.finish();
}

criterion_group!(benches, batch_circuit, recursive_circuit);
criterion_main!(benches);