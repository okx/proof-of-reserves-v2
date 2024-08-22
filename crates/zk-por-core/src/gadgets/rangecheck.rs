use plonky2::{
    field::types::{Field},
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::circuit_builder::CircuitBuilder,
};

use crate::{
    types::{D, F},
};

#[derive(Debug, Clone)]
pub struct RangeCheckTargets {
    pub value_target: Target,
    pub limbs: Vec<Target>,
}

impl RangeCheckTargets {
    pub fn new(builder: &mut CircuitBuilder<F, D>, lut_index: usize) -> Self {
        let value_target = builder.add_virtual_target();
        let low = builder.add_virtual_target();
        let mid = builder.add_virtual_target();
        let high = builder.add_virtual_target();


        builder.add_lookup_from_index(low, lut_index);
        builder.add_lookup_from_index(mid, lut_index);
        builder.add_lookup_from_index(high, lut_index);

        let pow2 = builder.constant(F::from_canonical_u64(1 << 16));
        let pow3 = builder.constant(F::from_canonical_u64(1 << 32));
        let mut sum = builder.mul_add(mid, pow2, low);
        sum = builder.mul_add(high, pow3, sum);

        builder.connect(value_target, sum);
        RangeCheckTargets { value_target, limbs: vec![low, mid, high] }
    }

    pub fn set_targets(&self, value: F, limbs: &[F], pw: &mut PartialWitness<F>) {
        pw.set_target(self.value_target, value);
        pw.set_target_arr(&self.limbs, limbs);
    }
}


#[cfg(test)]
pub mod test {
    use plonky2::field::types::{Field, Field64};
    use plonky2_field::goldilocks_field::GoldilocksField;

    use crate::{
        circuit_utils::run_circuit_test,
        parser::{FileManager, JsonFileManager},
        U16_TABLE
    };

    use super::RangeCheckTargets;

    #[test]
    fn test_rangecheck_target() {
        println!("u16 len: {:?}", U16_TABLE[65535]);
        run_circuit_test(|builder, pw| {
            let rangecheck_target = RangeCheckTargets::new(builder);
            let inner = u64::from_le_bytes([1, 1, 2, 2, 3, 3, 0, 0]);
            let low = inner & 0xFFFF;
            let high = inner >> 32;
            let mid = (inner >> 16) & 0xFFFF;

            let pow2 = 1 << 16;
            let pow3 = 1 << 32;
            let mut sum = mid * pow2 + low;
            sum = high * pow3 + sum;
            assert_eq!(sum, inner);

            let val = GoldilocksField::from_canonical_u64(inner);

            let limbs = vec![
                GoldilocksField::from_canonical_u64(u64::from_le_bytes([1, 1, 0, 0, 0, 0, 0, 0])),
                GoldilocksField::from_canonical_u64(u64::from_le_bytes([2, 2, 0, 0, 0, 0, 0, 0])),
                GoldilocksField::from_canonical_u64(u64::from_le_bytes([3, 3, 0, 0, 0, 0, 0, 0])),
            ];
            rangecheck_target.set_targets(val, &limbs, pw);
        });
    }
}
