use plonky2::{
    hash::{
        hash_types::{HashOut, RichField},
        poseidon::PoseidonHash,
    },
    iop::witness::PartialWitness,
    plonk::{
        circuit_data::{CircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig, Hasher},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
};
use plonky2_field::extension::Extendable;

use tracing::error;

use crate::{
    circuit_utils::prove_timing,
    types::{D, F},
};

use super::recursive_circuit::RecursiveTargets;

pub struct RecursiveProver<C: GenericConfig<D, F = F>, const N: usize> {
    // pub batch_id: usize,
    pub sub_proofs: [ProofWithPublicInputs<F, C, D>; N],
    pub sub_circuit_vd: VerifierOnlyCircuitData<C, D>,
}

impl<C: GenericConfig<D, F = F>, const N: usize> RecursiveProver<C, N> {
    /// Get proof with a pre-compiled merkle sum circuit and recursive targets. In this method we do not need to build the circuit as we use a pre-built circuit.
    pub fn get_proof_with_circuit_data(
        &self,
        recursive_targets: RecursiveTargets<N>,
        cd: &CircuitData<F, C, D>,
    ) -> ProofWithPublicInputs<F, C, D>
    where
        <C as GenericConfig<2>>::Hasher: AlgebraicHasher<F>,
    {
        let mut pw = PartialWitness::<F>::new();
        let CircuitData { prover_only, common, .. } = &cd;

        recursive_targets.set_targets(&mut pw, self.sub_proofs.to_vec(), &self.sub_circuit_vd);

        let mut t = prove_timing();
        let proof_res = prove(prover_only, common, pw, &mut t);

        match proof_res {
            Ok(proof) => {
                let proof_verification_res = cd.verify(proof.clone());
                match proof_verification_res {
                    Ok(_) => proof,
                    Err(e) => {
                        error!("Proof verification failed: {:?}", e);
                        panic!("Proof verification failed!");
                    }
                }
            }
            Err(e) => {
                error!("Proof generation failed: {:?}", e);
                panic!("Proof generation failed!");
            }
        }
    }
}

pub fn hash_n_subhashes<F: RichField + Extendable<D>, const D: usize>(
    hashes: &Vec<HashOut<F>>,
) -> HashOut<F> {
    let inputs : Vec<F> = hashes.iter().map(|h|{h.elements.to_vec()}).flatten().collect();
    let hash = PoseidonHash::hash_no_pad(inputs.as_slice());
    hash
}