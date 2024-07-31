use log::Level;
use plonky2::{
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{GenericHashOut, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::goldilocks_field::GoldilocksField;
use zk_por_core::{
    circuits::{
        account_circuit::{AccountSumTargets, AccountTargets},
        circuit_config::STANDARD_CONFIG,
        merkle_sum_circuit::build_merkle_sum_tree_from_account_targets,
    },
    core::{account::Account, parser::read_json_into_accounts_vec},
    recursive::{prove::{aggregate_proofs_at_level, prove_n_subproofs}, vd::VdTree},
    types::{C, D, F},
};
pub type ProofTuple<F, C, const D: usize> = (
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, D>,
);
use rand::Rng;

fn gen_batch_proof(accounts: Vec<&Account>) -> ProofTuple<F, C, D> {
    println!("Num Accounts : {:?}", accounts.len(),);
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let mut pw = PartialWitness::<GoldilocksField>::new();
    let mut account_targets: Vec<AccountTargets> = Vec::new();

    for i in 0..accounts.len() {
        let asset_targets = builder.add_virtual_targets(accounts.get(i).unwrap().assets.len());
        let debt_targets = builder.add_virtual_targets(accounts.get(i).unwrap().debt.len());
        let account_target = AccountTargets {
            assets: asset_targets,
            debt: debt_targets,
        };

        account_target.set_account_targets(accounts.get(i).unwrap(), &mut pw);
        account_targets.push(account_target);
    }

    // println!("Account Targets Created");

    let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
        .iter()
        .map(|x| AccountSumTargets::from_account_target(x, &mut builder))
        .collect();
    let merkle_tree_targets =
        build_merkle_sum_tree_from_account_targets(&mut builder, &mut account_sum_targets);

    builder.print_gate_counts(0);
    let mut timing = TimingTree::new("prove", Level::Debug);
    let start = std::time::Instant::now();
    let data = builder.build::<C>();
    println!("Built Circuit, took: {:?}", start.elapsed().as_millis());
    let CircuitData {
        prover_only,
        common,
        verifier_only: _,
    } = &data;
    println!("Started Proving");
    let start = std::time::Instant::now();
    let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);
    println!("proving, took: {:?}", start.elapsed().as_millis());
    let proof = proof_res.expect("Proof failed");

    // let proof_verification_res = data.verify(proof.clone());
    (proof, data.verifier_only, data.common)
}

fn gen_random(max: u32) -> usize {
    let mut rng = rand::thread_rng();
    let x: u32 = rng.gen_range(0..max);
    usize::try_from(x).unwrap()
}

fn gen_random_range<const N: usize>(max: u32) -> [usize; N] {
    let mut range = [0; N];
    for i in 0..N {
        range[i] = gen_random(max);
    }
    range
}

fn get_random_account_batch<const N: usize>(accounts: &[Account]) -> Vec<&Account> {
    let range = gen_random_range::<N>(u32::try_from(accounts.len()).unwrap());
    let mut account_slice = Vec::<&Account>::new();
    for i in 0..N {
        account_slice.push(&accounts[range[i]])
    }
    account_slice
}

fn main() {

    let path = "test-data/batch0.json";
    let accounts = read_json_into_accounts_vec(path);
    println!("Num Accounts : {:?}", accounts.len(),);

    const ACCOUNT_BATCH: usize = 2;
    
    const N: usize = 8;
    let proofs = (0..N).map(|_| {
        let batch_account = get_random_account_batch::<ACCOUNT_BATCH>(&accounts);
        gen_batch_proof(batch_account)
    }).collect::<Vec<ProofTuple<F, C, D>>>();
    let _ = prove_n_subproofs::<C, C, N>(
        &proofs,
        &STANDARD_CONFIG,
    );
}
