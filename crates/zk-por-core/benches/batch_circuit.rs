#![feature(test)]

use zk_por_core::{
    account::gen_accounts_with_random_data, merkle_sum_prover::prover::MerkleSumTreeProver,
};

extern crate test;
use test::Bencher;

fn bench(b: &mut Bencher, batch_size: usize) {
    let num_assets = 50;
    let accounts = gen_accounts_with_random_data(batch_size, num_assets);
    let prover = MerkleSumTreeProver { accounts };
    b.iter(|| _ = prover.get_proof());
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
