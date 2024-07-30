use log::Level;
use plonky2::{iop::witness::PartialWitness, plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData, prover::prove}, util::timing::TimingTree};
use plonky2_field::goldilocks_field::GoldilocksField;
use zk_por_core::{circuits::{account_circuit::{AccountSumTargets, AccountTargets}, circuit_config::STANDARD_CONFIG, merkle_sum_circuit::build_merkle_sum_tree_from_account_targets}, core::parser::read_json_into_accounts_vec, types::{C, D, F}};

pub fn main(){
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let mut pw = PartialWitness::<GoldilocksField>::new();

    let path = "test-data/batch0.json";
    let accounts = read_json_into_accounts_vec(path);
    println!("Num Accounts : {:?}", accounts.len());
    let mut account_targets: Vec<AccountTargets> = Vec::new();

    for i in 0..accounts.len(){
        let equity_targets = builder.add_virtual_targets(accounts.get(i).unwrap().equity.len());
        let debt_targets = builder.add_virtual_targets(accounts.get(i).unwrap().debt.len());
        let account_target = AccountTargets{
            equity: equity_targets,
            debt: debt_targets,
        };

        account_target.set_account_targets(accounts.get(i).unwrap(), &mut pw);
        account_targets.push(
            account_target
        );
    }

    println!("Account Targets Created");

    let mut account_sum_targets: Vec<AccountSumTargets> = account_targets.iter().map(|x| AccountSumTargets::from_account_target(x, &mut builder)).collect();
    let merkle_tree_targets = build_merkle_sum_tree_from_account_targets(&mut builder, &mut account_sum_targets);

    println!("Merkle Tree Created");

    builder.print_gate_counts(0);
    let mut timing = TimingTree::new("prove", Level::Debug);
    let data = builder.build::<C>();
    println!("Built Circuit");
    let CircuitData { prover_only, common, verifier_only: _ } = &data;
    println!("Started Proving");
    let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);
    let proof = proof_res.expect("Proof failed");

    println!("PROOF: {:?}", proof);
    println!("Finished Proving");

    println!("Verifying Proof");
    // Verify proof
    let proof_verification_res = data.verify(proof.clone());
    println!("Proof Verified");
}