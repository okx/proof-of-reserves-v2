use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

// Extension of size 2
pub const D: usize = 2;

// Constrict our values to 62 bits.
pub const MAX_POSITIVE_AMOUNT_LOG: usize = 62;

pub type C = PoseidonGoldilocksConfig;
pub type F = <C as GenericConfig<D>>::F;

