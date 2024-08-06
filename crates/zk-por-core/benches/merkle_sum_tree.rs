#![feature(test)]
use plonky2::plonk::circuit_builder::CircuitBuilder;
use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::{circuits::account_circuit::AccountTargets, prover::MerkleSumTreeProver},
    merkle_sum_tree::MerkleSumTree,
    types::{C, D, F},
};

extern crate test;
use test::Bencher;

fn bench(b: &mut Bencher, batch_size: usize) {
    let num_assets = 200;
    let accounts = gen_accounts_with_random_data(batch_size, num_assets);

    b.iter(|| _ = MerkleSumTree::new_tree_from_accounts(&accounts));
}

#[bench]
pub fn bench_batch_size_equal_1024(b: &mut Bencher) {
    bench(b, 1 << 10);
}

#[bench]
pub fn bench_batch_size_equal_2048(b: &mut Bencher) {
    bench(b, 1 << 11);
}

#[bench]
pub fn bench_batch_size_equal_4096(b: &mut Bencher) {
    bench(b, 1 << 12);
}

#[bench]
pub fn bench_batch_size_equal_8192(b: &mut Bencher) {
    bench(b, 1 << 13);
}

#[bench]
pub fn bench_batch_size_equal_16384(b: &mut Bencher) {
    bench(b, 1 << 14);
}

#[bench]
pub fn bench_batch_size_equal_32768(b: &mut Bencher) {
    bench(b, 1 << 15);
}
