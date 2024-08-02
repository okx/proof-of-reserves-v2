#![feature(test)]

use plonky2::plonk::proof::ProofWithPublicInputs;
use zk_por_core::merkle_sum_prover::prover::MerkleSumTreeProver;

use zk_por_core::{
    account::gen_accounts_with_random_data,
    recursive_prover::prover::RecursiveProver,
    types::{C, D, F},
};

extern crate test;
use test::Bencher;

fn bench<const SUBPROOF_NUM: usize>(b: &mut Bencher, batch_size: usize) {
    let asset_num = 50;
    let accounts = gen_accounts_with_random_data(batch_size, asset_num);
    let prover = MerkleSumTreeProver { accounts };

    let (merkle_sum_proof, merkle_sum_cd) = prover.get_proof_with_cd();

    let sub_proofs: [ProofWithPublicInputs<F, C, D>; SUBPROOF_NUM] =
        std::array::from_fn(|_| merkle_sum_proof.clone());

    let recursive_prover = RecursiveProver { sub_proofs, merkle_sum_circuit: merkle_sum_cd };

    b.iter(|| {
        recursive_prover.get_proof();
    });
}
#[bench]
pub fn bench_subproof_num_4_batch_size_1024(b: &mut Bencher) {
    bench::<4>(b, 1024);
}

#[bench]
pub fn bench_subproof_num_8_batch_size_1024(b: &mut Bencher) {
    bench::<8>(b, 1024);
}

#[bench]
pub fn bench_subproof_num_16_batch_size_1024(b: &mut Bencher) {
    bench::<16>(b, 1024);
}

#[bench]
pub fn bench_subproof_num_32_batch_size_1024(b: &mut Bencher) {
    bench::<32>(b, 1024);
}

#[bench]
pub fn bench_subproof_num_64_batch_size_1024(b: &mut Bencher) {
    bench::<64>(b, 1024);
}
