use plonky2::plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs};
use zk_por_core::{
    account::gen_accounts_with_random_data, circuit_config::STANDARD_CONFIG,
    circuit_registry::registry::CircuitRegistry, e2e::batch_prove_accounts, types::F,
};

use zk_por_tracing::{init_tracing, TraceConfig};

#[test]
fn test_proof_serialization() {
    let cfg = TraceConfig {
        prefix: "zkpor".to_string(),
        dir: "logs".to_string(),
        level: tracing::Level::DEBUG,
        console: true,
        flame: false,
    };

    {
        init_tracing(cfg)
    };

    const RECURSION_BRANCHOUT_NUM: usize = 4;
    let batch_size = 8;
    let token_num = 4;

    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        STANDARD_CONFIG,
        vec![STANDARD_CONFIG; 2],
    );
    let circuit_common_data_reference = &circuit_registry.get_batch_circuit().0.common;

    let proving_thread_num = 2;

    let accounts = gen_accounts_with_random_data(batch_size * 3, 4);

    let proofs = batch_prove_accounts(&circuit_registry, accounts, proving_thread_num, batch_size);

    proofs.iter().for_each(|proof| {
        let proof_bytes = proof.to_bytes();
        let deser_proof = ProofWithPublicInputs::<F, PoseidonGoldilocksConfig, 2>::from_bytes(
            proof_bytes,
            circuit_common_data_reference,
        )
        .expect("Failed to deserialize proof!");
        assert_eq!(deser_proof, *proof);
    });
}
