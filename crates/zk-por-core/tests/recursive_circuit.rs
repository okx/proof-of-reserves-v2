use zk_por_core::recursive::{prover::prove_n_subproofs};

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

    let start = std::time::Instant::now();
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num);
    println!("build merkle sum tree circuit in : {:?}", start.elapsed());

    let (accounts, equity_sum, debt_sum) = gen_accounts_with_random_data(batch_size, asset_num);
    let prover = MerkleSumTreeProver { accounts };

    let start = std::time::Instant::now();
    let merkle_sum_proof = prover.prove_with_circuit(&merkle_sum_circuit, account_targets).unwrap();
    println!("prove merkle sum tree in : {:?}", start.elapsed());

    let start = std::time::Instant::now();
    let (recursive_circuit, recursive_account_targets) =
        build_recursive_n_circuit::<C, RECURSIVE_FACTOR>(
            &merkle_sum_circuit.common,
            &merkle_sum_circuit.verifier_only,
        );
    println!("build recursive circuit in : {:?}", start.elapsed());

    let mut subproofs = Vec::new();
    (0..RECURSIVE_FACTOR).for_each(|_| {
        subproofs.push(merkle_sum_proof.clone());
    });
    let start = std::time::Instant::now();
    let recursive_proof_result = prove_n_subproofs(
        subproofs,
        &merkle_sum_circuit.verifier_only,
        &recursive_circuit,
        recursive_account_targets,
    );
    println!("prove recursive subproofs in : {:?}", start.elapsed());

    assert!(recursive_proof_result.is_ok());
    let recursive_proof = recursive_proof_result.unwrap();

    // print public inputs in recursive proof
    assert_eq!(
        F::from_canonical_u32(equity_sum * (RECURSIVE_FACTOR as u32)),
        recursive_proof.public_inputs[0]
    );
    assert_eq!(
        F::from_canonical_u32(debt_sum * (RECURSIVE_FACTOR as u32)),
        recursive_proof.public_inputs[1]
    );

    assert!(recursive_circuit.verify(recursive_proof).is_ok());
}
