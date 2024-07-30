use std::panic;
use log::Level;
use plonky2::{hash::hash_types::{HashOutTarget, RichField}, iop::{target::{BoolTarget, Target}, witness::PartialWitness}, plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData, config::GenericConfig, prover::prove}, util::timing::TimingTree};
use plonky2_field::extension::Extendable;

use crate::{circuits::circuit_config::STANDARD_CONFIG, types::{C, MAX_POSITIVE_AMOUNT_LOG}};

/// Test runner for ease of testing
pub fn run_circuit_test<T, F, const D: usize>(test: T) -> ()
where
    T: FnOnce(&mut CircuitBuilder<F, D>, &mut PartialWitness<F>) -> () + panic::UnwindSafe,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let mut pw: PartialWitness<F> = PartialWitness::<F>::new();
    test(&mut builder, &mut pw);
    builder.print_gate_counts(0);
    let mut timing = TimingTree::new("prove", Level::Debug);
    let data = builder.build::<C>();
    let CircuitData { prover_only, common, verifier_only: _ } = &data;
    let proof = prove(&prover_only, &common, pw, &mut timing).expect("Prove fail");
    timing.print();
    data.verify(proof).expect("Verify fail")
}

/// Computes `if b { h0 } else { h1 }`.
pub fn select_hash<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b: BoolTarget,
    h0: HashOutTarget,
    h1: HashOutTarget,
) -> HashOutTarget {
    HashOutTarget {
        elements: core::array::from_fn(|i| builder.select(b, h0.elements[i], h1.elements[i])),
    }
}

/// Assert 0 <= x <= MAX_POSITIVE_AMOUNT
/// MAX_POSITIVE_AMOUNT =  (1 << MAX_POSITIVE_AMOUNT_LOG) - 1
pub fn assert_non_negative_unsigned<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    x: Target,
) {
    builder.range_check(x, MAX_POSITIVE_AMOUNT_LOG);
}
