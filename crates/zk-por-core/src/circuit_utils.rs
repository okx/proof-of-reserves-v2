use log::Level;
use plonky2::{
    hash::hash_types::RichField,
    iop::{target::Target, witness::PartialWitness},
    plonk::{
        circuit_builder::CircuitBuilder, circuit_data::CircuitData, config::GenericConfig,
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::extension::Extendable;
use std::panic;

use crate::{
    circuit_config::STANDARD_CONFIG,
    types::{C, D, F, MAX_POSITIVE_AMOUNT_LOG},
};

pub fn prove_timing() -> TimingTree {
    let mut level = Level::Info;
    if cfg!(debug_assertions) {
        level = Level::Debug;
    }

    TimingTree::new("prove", level)
}

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

/// Assert 0 <= x <= MAX_POSITIVE_AMOUNT
/// MAX_POSITIVE_AMOUNT =  (1 << MAX_POSITIVE_AMOUNT_LOG) - 1
pub fn assert_non_negative_unsigned(builder: &mut CircuitBuilder<F, D>, x: Target) {
    builder.range_check(x, MAX_POSITIVE_AMOUNT_LOG);
}

#[cfg(test)]
pub mod test {
    use crate::types::F;

    use plonky2_field::types::{Field, Field64};

    use super::{assert_non_negative_unsigned, run_circuit_test};

    #[test]
    fn test_assert_non_negative_unsigned() {
        run_circuit_test(|builder, _pw| {
            let x = builder.constant(F::from_canonical_u16(0));
            assert_non_negative_unsigned(builder, x);
        });
    }

    #[test]
    #[should_panic]
    fn test_assert_non_negative_unsigned_panic() {
        run_circuit_test(|builder, _pw| {
            let x = builder.constant(F::from_canonical_i64(-1));
            assert_non_negative_unsigned(builder, x);
        });
    }
}
