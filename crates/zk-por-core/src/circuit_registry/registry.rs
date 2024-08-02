use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::{
        CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
    },
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputsTarget,
};

use crate::{
    circuit_config::STANDARD_CONFIG,
    merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget,
};

use crate::types::{C, D, F};



struct CircuitRegistry {

}

impl CircuitRegistry {
	pub fn new() -> Self {
		CircuitRegistry {}
	}

	pub fn get_batch_circuit(&self) -> CircuitData<F, C, D> {
	}

	pub fn get_empty_batch_circuit_proof(&self) -> ProofWithPublicInputsTarget<D> {
	}

	// leaf node at level 0
	pub fn get_recursive_circuit(&self, level: usize) -> CircuitData<F, C, D> {
	}

	pub fn get_empty_recursive_circuit_proof(&self, level: usize) -> ProofWithPublicInputsTarget<D> {
	}
}