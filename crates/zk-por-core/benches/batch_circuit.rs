#![feature(test)]
use plonky2::plonk::circuit_builder::CircuitBuilder;
use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::{circuits::account_circuit::AccountTargets, prover::MerkleSumTreeProver},
    types::{C, D, F},
};

extern crate test;
use test::Bencher;

fn bench(b: &mut Bencher, batch_size: usize) {
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let num_assets = 50;
    let accounts = gen_accounts_with_random_data(batch_size, num_assets);
    let prover = MerkleSumTreeProver { accounts };
    let account_targets: Vec<AccountTargets> = prover.build_merkle_tree_targets(&mut builder);
    let data = &builder.build::<C>();

    b.iter(|| _ = prover.get_proof_with_circuit_data(account_targets.as_slice(), data));
}

#[bench]
pub fn bench_batch_size_equal_2(b: &mut Bencher) {
    bench(b, 2);
}

#[bench]
pub fn bench_batch_size_equal_16(b: &mut Bencher) {
    bench(b, 16);
}

#[bench]
pub fn bench_batch_size_equal_256(b: &mut Bencher) {
    bench(b, 256);
}

#[bench]
pub fn bench_batch_size_equal_1024(b: &mut Bencher) {
    bench(b, 1024);
}

#[bench]
pub fn bench_batch_size_equal_2048(b: &mut Bencher) {
    bench(b, 2048);
}

#[bench]
pub fn bench_batch_size_equal_4096(b: &mut Bencher) {
    bench(b, 4096);
}
