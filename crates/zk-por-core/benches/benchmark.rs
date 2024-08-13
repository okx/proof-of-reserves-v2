use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode,
};
use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::{
        circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
    },
    recursive_prover::{prover::RecursiveProver, recursive_circuit::build_recursive_n_circuit},
    types::{C, D, F},
};

use plonky2::plonk::proof::ProofWithPublicInputs;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Benchmark the batch proving of the accounts. There are {parallism} threads, each thread proving the batch_size accounts.
pub fn bench_batch_circuit(
    c: &mut BenchmarkGroup<WallTime>,
    batch_size: usize,
    num_assets: usize,
    parallism: usize,
) {
    let accounts = gen_accounts_with_random_data(batch_size, num_assets);
    let bench_id =
        format!("batch_circuit_{}_asset_num_{}_parallism_{}", batch_size, num_assets, parallism);
    let (circuit_data, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, num_assets, STANDARD_CONFIG);
    c.bench_function(bench_id.as_str(), |b| {
        b.iter(|| {
            (0..parallism).into_par_iter().for_each(|_| {
                let prover = MerkleSumTreeProver { accounts: accounts.clone() };
                let _ = prover.get_proof_with_circuit_data(&account_targets, &circuit_data);
            });
        })
    });
}

pub fn batch_circuit_for_batch_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_circuit_for_batch_size");
    group.sample_size(10);
    let num_assets = 200;
    let parallism = 1;
    for &batch_size in [16, 64, 256, 512, 1024].iter() {
        bench_batch_circuit(&mut group, batch_size, num_assets, parallism);
    }
    group.finish();
}

pub fn batch_circuit_for_asset_num(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_circuit_for_asset_num");
    group.sample_size(10);
    let batch_size = 1024;
    let parallism = 1;
    for &num_assets in [4, 20, 50, 100, 200].iter() {
        bench_batch_circuit(&mut group, batch_size, num_assets, parallism);
    }
    group.finish();
}

pub fn batch_circuit_for_parallism(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_circuit_for_parallism");
    group.sample_size(10);
    let asset_num = 200;
    for &batch_size in [16, 1024].iter() {
        for &parallism in [1, 2, 4, 8, 16, 32].iter() {
            bench_batch_circuit(&mut group, batch_size, asset_num, parallism);
        }
    }
    group.finish();
}

pub fn bench_recursive_circuit<const SUBPROOF_NUM: usize>(
    g: &mut BenchmarkGroup<WallTime>,
    parallism: usize,
) {
    let batch_size = 1024;
    let asset_num = 4;
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num, STANDARD_CONFIG);

    let accounts = gen_accounts_with_random_data(batch_size, asset_num);
    let prover = MerkleSumTreeProver { accounts };

    let merkle_sum_proof =
        prover.get_proof_with_circuit_data(&account_targets, &merkle_sum_circuit);

    let (recursive_circuit, recursive_targets) = build_recursive_n_circuit::<C, SUBPROOF_NUM>(
        &merkle_sum_circuit.common,
        &merkle_sum_circuit.verifier_only,
        STANDARD_CONFIG,
    );

    let subproofs: [ProofWithPublicInputs<F, C, D>; SUBPROOF_NUM] =
        std::array::from_fn(|_| merkle_sum_proof.clone());
    let bench_id = format!("recursive_circuit_{}_parallism_{}", SUBPROOF_NUM, parallism);
    g.bench_function(bench_id.as_str(), |b| {
        b.iter(|| {
            (0..parallism).into_par_iter().for_each(|_| {
                let recursive_prover = RecursiveProver {
                    sub_proofs: subproofs.clone(),
                    sub_circuit_vd: merkle_sum_circuit.verifier_only.clone(),
                };
                recursive_prover
                    .get_proof_with_circuit_data(recursive_targets.clone(), &recursive_circuit);
            });
        })
    });
}

pub fn recursive_circuit_for_branchout(c: &mut Criterion) {
    let mut group = c.benchmark_group("recursive_circuit_for_branchout");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat); // for long running benchmarks
    let parallism = 1;
    bench_recursive_circuit::<4>(&mut group, parallism);
    bench_recursive_circuit::<8>(&mut group, parallism);
    bench_recursive_circuit::<16>(&mut group, parallism);
    bench_recursive_circuit::<32>(&mut group, parallism);
    bench_recursive_circuit::<64>(&mut group, parallism);
    group.finish();
}

pub fn recursive_circuit_for_parallism(c: &mut Criterion) {
    let mut group = c.benchmark_group("recursive_circuit_for_parallism");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat); // for long running benchmarks
    const SUBPROOF_NUM: usize = 64;
    for &parallism in [1, 2, 4].iter() {
        bench_recursive_circuit::<SUBPROOF_NUM>(&mut group, parallism);
    }
}

criterion_group!(
    benches,
    batch_circuit_for_asset_num,
    batch_circuit_for_batch_size,
    batch_circuit_for_parallism,
    recursive_circuit_for_branchout,
    recursive_circuit_for_parallism
);
criterion_main!(benches);
