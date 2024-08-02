use plonky2::plonk::proof::ProofWithPublicInputs;
use zk_por_core::{recursive_prover::prover::RecursiveProver, types::D};

use zk_por_core::merkle_sum_prover::prover::MerkleSumTreeProver;

use plonky2_field::types::Field;
use zk_por_core::{
    account::gen_accounts_with_random_data,
    types::{C, F},
};

#[test]
fn test() {
    let batch_size = 4;
    let asset_num = 2;
    const RECURSIVE_FACTOR: usize = 8;

    let accounts = gen_accounts_with_random_data(batch_size, asset_num);

    let equity_sum = accounts
        .iter()
        .fold(F::ZERO, |acc, x| acc + x.equity.iter().fold(F::ZERO, |acc_2, y| acc_2 + *y));

    let debt_sum = accounts
        .iter()
        .fold(F::ZERO, |acc, x| acc + x.debt.iter().fold(F::ZERO, |acc_2, y| acc_2 + *y));

    let prover = MerkleSumTreeProver { accounts };

    let (merkle_sum_proof, cd) = prover.get_proof_with_cd();

    let sub_proofs: [ProofWithPublicInputs<F, C, D>; RECURSIVE_FACTOR] =
        std::array::from_fn(|_| merkle_sum_proof.clone());

    let recursive_prover = RecursiveProver { sub_proofs, merkle_sum_circuit: cd };

    let recursive_proof_result = recursive_prover.get_proof();

    // print public inputs in recursive proof
    assert_eq!(
        equity_sum * F::from_canonical_u32(RECURSIVE_FACTOR as u32),
        recursive_proof_result.public_inputs[0]
    );
    assert_eq!(
        debt_sum * F::from_canonical_u32(RECURSIVE_FACTOR as u32),
        recursive_proof_result.public_inputs[1]
    );
}
