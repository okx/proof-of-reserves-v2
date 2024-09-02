use plonky2::plonk::{
    circuit_data::{CommonCircuitData, VerifierOnlyCircuitData},
    config::{GenericConfig, PoseidonGoldilocksConfig},
    proof::ProofWithPublicInputs,
};

// Extension of size 2
pub const D: usize = 2;

// Constrict our values to 62 bits.
pub const MAX_POSITIVE_AMOUNT_LOG: usize = 62;

// Number of accounts in one merkle sum tree batch.
pub const MERKLE_SUM_TREE_BATCH_SIZE: usize = 1;

pub type C = PoseidonGoldilocksConfig;
pub type F = <C as GenericConfig<D>>::F;

pub type ProofTuple<F, C, const D: usize> =
    (ProofWithPublicInputs<F, C, D>, VerifierOnlyCircuitData<C, D>, CommonCircuitData<F, D>);
