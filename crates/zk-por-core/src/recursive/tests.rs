use log::Level;
use plonky2::{
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{GenericHashOut, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};
use super::super::{
    circuits::{
        account_circuit::{AccountSumTargets, AccountTargets},
        circuit_config::STANDARD_CONFIG,
        merkle_sum_circuit::build_merkle_sum_tree_from_account_targets,
    },
    core::{account::Account, parser::read_json_into_accounts_vec},
    // recursive::{prove::{aggregate_proofs_at_level, prove_n_subproofs}, vd::VdTree},
    types::{C, D, F},
};

use plonky2::iop::target::Target;
use plonky2_field::types::Field;
use plonky2_field::goldilocks_field::GoldilocksField;
use rand::Rng;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CIRCUIT : (CircuitData<F,C,D>, Target, Target, Target) = {
        let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
        let t1 = builder.add_virtual_target();
        let t2 = builder.add_virtual_target();
        let t3 = builder.add_virtual_target();

        let t4 = builder.add(t1, t2);
        let t5 = builder.mul(t3, t4);
        builder.register_public_input(t5);

        let circuit_data = builder.build::<C>();
        (circuit_data, t1.clone(), t2.clone(), t3.clone())
    };
}

#[test]
fn test_one() {
    let circuit_data = &CIRCUIT.0;
    let t1 = CIRCUIT.1;
    let t2 = CIRCUIT.2;
    let t3 = CIRCUIT.3;

    let pd = &circuit_data.prover_only;
    let vd = &circuit_data.verifier_only;
    let cd = &circuit_data.common;
    let mut timing = TimingTree::new("prove", Level::Debug);

    let mut pw1: PartialWitness<F> = PartialWitness::new();
    pw1.set_target(t1, F::ONE);
    pw1.set_target(t2, F::TWO);
    pw1.set_target(t3, F::from_canonical_u16(3));

    let proof_res1 = prove(pd, cd, pw1, &mut timing).unwrap();
    print!("pub inputs1: {:?}", proof_res1.public_inputs);



    ////////////////////////////////////////////////////////////////
    let circuit_data = &CIRCUIT.0;
    let t1 = CIRCUIT.1;
    let t2 = CIRCUIT.2;
    let t3 = CIRCUIT.3;

    let pd = &circuit_data.prover_only;
    let vd = &circuit_data.verifier_only;
    let cd = &circuit_data.common;
    let mut timing = TimingTree::new("prove", Level::Debug);

    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);

    let mut pw1: PartialWitness<F> = PartialWitness::new();
    pw1.set_target(t1, F::from_canonical_u16(3));
    pw1.set_target(t2, F::TWO);
    pw1.set_target(t3, F::from_canonical_u16(1));

    let proof_res1 = prove(pd, cd, pw1, &mut timing).unwrap();
    print!("pub inputs2: {:?}", proof_res1.public_inputs);
}