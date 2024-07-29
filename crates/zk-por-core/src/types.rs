use plonky2::{
    field::{extension::quadratic::QuadraticExtension, goldilocks_field::GoldilocksField},
    hash::poseidon_bn128::PoseidonBN128Hash,
    plonk::config::{GenericConfig, PoseidonGoldilocksConfig},
};

// Extension of size 2
pub const D: usize = 2;

// Constrict our values to 62 bits.
pub const MAX_POSITIVE_AMOUNT_LOG: usize = 62;

/// Configuration using Poseidon over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PoseidonBN128GoldilocksConfig;

impl GenericConfig<D> for PoseidonBN128GoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<F>;
    type Hasher = PoseidonBN128Hash;
    type InnerHasher = PoseidonBN128Hash;
}

pub type C = PoseidonGoldilocksConfig;
pub type F = <C as GenericConfig<D>>::F;

