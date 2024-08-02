use plonky2::{
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};

use crate::types::{D, F};
use anyhow::Result;

use super::recursive_circuit::{build_new_recursive_n_circuit_targets, RecursiveTargets};

pub struct RecursiveProver<
    C: GenericConfig<D, F = F>,
    const N: usize,
> {
    pub batch_id: usize,
    pub sub_proofs: [ProofWithPublicInputs<F, C, D>; N],
    pub merkle_sum_circuit: CircuitData<F, C, D>,
}

impl<C: GenericConfig<D, F = F>, const N: usize>
    RecursiveProver<C, N>
{
    pub fn build_and_set_recursive_targets(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        pw: &mut PartialWitness<F>,
    ) where <C as GenericConfig<2>>::Hasher: AlgebraicHasher<F>{

        let recursive_targets: RecursiveTargets<N> = build_new_recursive_n_circuit_targets(
            &self.merkle_sum_circuit.common,
            &self.merkle_sum_circuit.verifier_only,
            builder
        );

        recursive_targets.set_targets(pw, self.sub_proofs.to_vec(), &self.merkle_sum_circuit.verifier_only)
    }
}

pub fn prove_n_subproofs<
    C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const N: usize,
>(
    sub_proofs: Vec<ProofWithPublicInputs<F, InnerC, D>>,
    inner_circuit_vd: &VerifierOnlyCircuitData<InnerC, D>,
    recursive_circuit: &CircuitData<F, C, D>,
    recursive_targets: RecursiveTargets<N>,
) -> Result<ProofWithPublicInputs<F, C, D>>
where
    InnerC::Hasher: AlgebraicHasher<F>,
    // [(); C::Hasher::HASH_SIZE]:, // TODO: figure out how to make this work
{
    // tracing::debug!("before build recurisve {} circuit", N);
    // let circuit_data = builder.build::<C>();
    // tracing::debug!("after build recurisve {} circuit", N);
    if sub_proofs.len() != N {
        return Err(anyhow::anyhow!(format!(
            "number of proofs [{}] is not consistent with N [{}]",
            sub_proofs.len(),
            N
        )));
    }

    let mut pw = PartialWitness::new();
    pw.set_verifier_data_target(&recursive_targets.verifier_circuit_target, inner_circuit_vd);

    (0..N).for_each(|i| {
        pw.set_proof_with_pis_target(
            &recursive_targets.proof_with_pub_input_targets[i],
            &sub_proofs[i],
        );
    });

    let mut timing = TimingTree::new("prove_N_subproofs", log::Level::Debug);
    #[cfg(not(debug_assertions))]
    let mut timing = TimingTree::new("prove_N_subproofs", log::Level::Info);

    let start = std::time::Instant::now();
    log::debug!("before prove");
    let proof = prove(&recursive_circuit.prover_only, &recursive_circuit.common, pw, &mut timing)?;
    log::debug!("time for {:?} proofs, {:?}", N, start.elapsed().as_millis());

    #[cfg(debug_assertions)]
    {
        recursive_circuit.verify(proof.clone())?;
    }

    Ok(proof)
}
