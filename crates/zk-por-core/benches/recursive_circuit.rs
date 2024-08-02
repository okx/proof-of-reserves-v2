#![feature(test)]

use zk_por_core::merkle_sum_prover::{
    circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
};

use zk_por_core::{
    account::gen_accounts_with_random_data,
    recursive::{circuit::build_recursive_n_circuit, prove::prove_n_subproofs},
    types::C,
};

extern crate test;
use test::Bencher;

fn bench<const SUBPROOF_NUM: usize>(b: &mut Bencher, batch_size: usize) {
    let asset_num = 4;
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num);

    let accounts = gen_accounts_with_random_data(batch_size, asset_num).0;
    let prover = MerkleSumTreeProver { accounts };

    let merkle_sum_proof = prover.prove_with_circuit(&merkle_sum_circuit, account_targets).unwrap();

    let (recursive_circuit, recursive_account_targets) = build_recursive_n_circuit::<C, SUBPROOF_NUM>(
        &merkle_sum_circuit.common,
        &merkle_sum_circuit.verifier_only,
    );
    let mut subproofs = Vec::new();
    (0..SUBPROOF_NUM).for_each(|_| {
        subproofs.push(merkle_sum_proof.clone());
    });

    b.iter(|| {
        _ = prove_n_subproofs(
            subproofs.clone(),
            &merkle_sum_circuit.verifier_only,
            &recursive_circuit,
            recursive_account_targets.clone(),
        );
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
