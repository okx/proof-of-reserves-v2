use plonky2_field::types::Field;
use zk_por_core::{
    account::gen_accounts_with_random_data,
    merkle_sum_prover::{
        circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
    },
    types::F,
};

#[test]
pub fn test_separate_circuit_building_and_proving() {
    let num_accounts = 10;
    let num_assets = 5;
    let (circuit_data, account_targets) = build_merkle_sum_tree_circuit(num_accounts, num_assets);

    let (accounts, equity_sum, debt_sum) = gen_accounts_with_random_data(num_accounts, num_assets);
    let prover = MerkleSumTreeProver { accounts };
    let proof_result = prover.prove_with_circuit(&circuit_data, account_targets);
    assert!(proof_result.is_ok());
    let proof = proof_result.unwrap();

    // account_sum and debt_sum are the public inputs
    assert_eq!(F::from_canonical_u32(equity_sum), proof.public_inputs[0]);
    assert_eq!(F::from_canonical_u32(debt_sum), proof.public_inputs[1]);
    assert!(circuit_data.verify(proof).is_ok());
}
