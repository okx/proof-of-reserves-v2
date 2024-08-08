use zk_por_core::merkle_sum_prover::{
    circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
};

use plonky2::plonk::proof::ProofWithPublicInputs;

use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    recursive_prover::{prover::RecursiveProver, recursive_circuit::build_recursive_n_circuit},
    types::{C, D, F},
};

fn main() {
    const SUBPROOF_NUM: usize = 4; // configure this for bench.

    let batch_size = 1024;
    let asset_num = 4;
    let (merkle_sum_circuit, account_targets) =
        build_merkle_sum_tree_circuit(batch_size, asset_num, STANDARD_CONFIG);
    println!("build merkle sum tree circuit");

    let accounts = gen_accounts_with_random_data(batch_size, asset_num);
    let prover = MerkleSumTreeProver { accounts };

    let merkle_sum_proof = prover.get_proof_with_circuit_data(&account_targets, &merkle_sum_circuit);
    println!("prove merkle sum tree");

    let (recursive_circuit, recursive_targets) = build_recursive_n_circuit::<C, SUBPROOF_NUM>(
        &merkle_sum_circuit.common,
        &merkle_sum_circuit.verifier_only,
        STANDARD_CONFIG,
    );
    println!("build recursive circuit");

    let subproofs: [ProofWithPublicInputs<F, C, D>; SUBPROOF_NUM] =
        std::array::from_fn(|_| merkle_sum_proof.clone());

    let recursive_prover = RecursiveProver {
        sub_proofs: subproofs,
        sub_circuit_vd: merkle_sum_circuit.verifier_only.clone(),
    };

    let start = std::time::Instant::now();
    recursive_prover.get_proof_with_circuit_data(recursive_targets, &recursive_circuit);
    println!("prove recursive {} subproofs in : {:?}", SUBPROOF_NUM, start.elapsed());
}
