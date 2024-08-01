use log::Level;
use plonky2::{
    hash::{
        hash_types::{HashOutTarget, RichField},
        poseidon::PoseidonHash,
    },
    iop::{
        target::{BoolTarget, Target},
        witness::PartialWitness,
    },
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
pub fn select_hash(
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
pub fn assert_non_negative_unsigned(
    builder: &mut CircuitBuilder<F, D>,
    x: Target,
) {
    builder.range_check(x, MAX_POSITIVE_AMOUNT_LOG);
}

/// Get Hash target by doing a poseidon hash on my input vector.
pub fn get_hash_from_input_targets_circuit(
    builder: &mut CircuitBuilder<F, D>,
    inputs: Vec<Target>,
) -> HashOutTarget {
    builder.hash_n_to_hash_no_pad::<PoseidonHash>(inputs)
}

/// Hash 2 hashout targets by splitting it into its individual component elements
pub fn hash_2_subhashes_circuit(
    builder: &mut CircuitBuilder<F, D>,
    hash_1: &HashOutTarget,
    hash_2: &HashOutTarget,
) -> HashOutTarget {
    let inputs = vec![hash_1.elements.to_vec(), hash_2.elements.to_vec()].concat();
    get_hash_from_input_targets_circuit(builder, inputs)
}

#[cfg(test)]
pub mod test {
    use crate::types::F;
    use plonky2::{hash::hash_types::HashOut, iop::witness::WitnessWrite};
    use plonky2_field::types::{Field, Field64};

    use super::{
        assert_non_negative_unsigned, get_hash_from_input_targets_circuit,
        hash_2_subhashes_circuit, run_circuit_test,
    };

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

    #[test]
    fn test_get_hash_from_input_targets_circuit() {
        run_circuit_test(|builder, pw| {
            let target_1 = builder.add_virtual_target();
            let target_2 = builder.add_virtual_target();
            let hash_target = builder.add_virtual_hash();
            let calculated_hash_target =
                get_hash_from_input_targets_circuit(builder, vec![target_1, target_2]);
            builder.connect_hashes(hash_target, calculated_hash_target);

            let value_1 = F::ZERO;
            let value_2 = F::ZERO;

            let hash = HashOut::from_vec(vec![
                F::from_canonical_u64(4330397376401421145),
                F::from_canonical_u64(14124799381142128323),
                F::from_canonical_u64(8742572140681234676),
                F::from_canonical_u64(14345658006221440202),
            ]);

            pw.set_target(target_1, value_1);
            pw.set_target(target_2, value_2);
            pw.set_hash_target(hash_target, hash);
        });
    }

    #[test]
    fn test_hash_2_subhashes_circuit() {
        run_circuit_test(|builder, pw| {
            let hash_target_1 = builder.add_virtual_hash();
            let hash_target_2 = builder.add_virtual_hash();
            let hash_target_3 = builder.add_virtual_hash();
            let calculated_hash_target =
                hash_2_subhashes_circuit(builder, &hash_target_1, &hash_target_2);
            builder.connect_hashes(hash_target_3, calculated_hash_target);

            let hash_1 = HashOut::from_vec(vec![
                F::from_canonical_u64(4330397376401421145),
                F::from_canonical_u64(14124799381142128323),
                F::from_canonical_u64(8742572140681234676),
                F::from_canonical_u64(14345658006221440202),
            ]);
            let hash_2 = HashOut::from_vec(vec![
                F::from_canonical_u64(4330397376401421145),
                F::from_canonical_u64(14124799381142128323),
                F::from_canonical_u64(8742572140681234676),
                F::from_canonical_u64(14345658006221440202),
            ]);

            let hash_3 = HashOut::from_vec(vec![
                F::from_canonical_u64(13121882728673923020),
                F::from_canonical_u64(10197653806804742863),
                F::from_canonical_u64(16037207047953124082),
                F::from_canonical_u64(2420399206709257475),
            ]);
            pw.set_hash_target(hash_target_1, hash_1);
            pw.set_hash_target(hash_target_2, hash_2);
            pw.set_hash_target(hash_target_3, hash_3);
        });
    }
}
