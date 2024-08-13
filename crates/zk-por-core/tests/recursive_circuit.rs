use plonky2::{hash::hash_types::HashOut, plonk::proof::ProofWithPublicInputs};
use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit,
    recursive_prover::{prover::RecursiveProver, recursive_circuit::RecursiveTargets},
    types::{C, D, F},
};

use zk_por_core::{
    merkle_sum_prover::prover::MerkleSumTreeProver,
    recursive_prover::{prover::hash_n_subhashes, recursive_circuit::build_recursive_n_circuit},
};

use plonky2_field::types::Field;

#[test]
fn test() {
    let batch_size = 4;
    let asset_num = 2;
    const RECURSION_BRANCHOUT_NUM: usize = 8;

    let accounts = gen_accounts_with_random_data(batch_size, asset_num);

    let equity_sum = accounts
        .iter()
        .fold(F::ZERO, |acc, x| acc + x.equity.iter().fold(F::ZERO, |acc_2, y| acc_2 + *y));

    let debt_sum = accounts
        .iter()
        .fold(F::ZERO, |acc, x| acc + x.debt.iter().fold(F::ZERO, |acc_2, y| acc_2 + *y));

    let start = std::time::Instant::now();
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num, STANDARD_CONFIG);
    println!("build merkle sum tree circuit in : {:?}", start.elapsed());

    let prover = MerkleSumTreeProver { accounts };

    let start = std::time::Instant::now();
    let merkle_sum_proof = prover.get_proof_with_circuit_data(account_targets.as_slice(), &merkle_sum_circuit);
    println!("prove merkle sum tree in : {:?}", start.elapsed());

    let sub_proofs: [ProofWithPublicInputs<F, C, D>; RECURSION_BRANCHOUT_NUM] =
        std::array::from_fn(|_| merkle_sum_proof.clone());

    let start = std::time::Instant::now();
    let (recursive_circuit, recursive_targets) =
        build_recursive_n_circuit::<C, RECURSION_BRANCHOUT_NUM>(
            &merkle_sum_circuit.common,
            &merkle_sum_circuit.verifier_only,
            STANDARD_CONFIG,
        );
    println!("build recursive N circuit in : {:?}", start.elapsed());

    let start = std::time::Instant::now();
    let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
    let subhashes = sub_proofs
        .clone()
        .map(|proof| HashOut::<F>::from_partial(&proof.public_inputs[hash_offset.clone()]));
    let recursive_prover = RecursiveProver {
        sub_proofs: sub_proofs,
        sub_circuit_vd: merkle_sum_circuit.verifier_only.clone(),
    };
    let recursive_proof_result =
        recursive_prover.get_proof_with_circuit_data(recursive_targets, &recursive_circuit);
    println!("prove recursive subproofs in : {:?}", start.elapsed());

    let expected_hash = hash_n_subhashes::<F, D>(&subhashes.to_vec());
    let actual_hash =
        HashOut::<F>::from_partial(&recursive_proof_result.public_inputs[hash_offset]);

    assert_eq!(expected_hash, actual_hash);

    // print public inputs in recursive proof
    assert_eq!(
        equity_sum * F::from_canonical_u32(RECURSION_BRANCHOUT_NUM as u32),
        recursive_proof_result.public_inputs[0]
    );
    assert_eq!(
        debt_sum * F::from_canonical_u32(RECURSION_BRANCHOUT_NUM as u32),
        recursive_proof_result.public_inputs[1]
    );
}
