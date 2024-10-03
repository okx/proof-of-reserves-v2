use plonky2::{
    plonk::circuit_data::VerifierCircuitData, util::serialization::DefaultGateSerializer,
};
use plonky2_field::types::Field;
use zk_por_core::{
    circuit_registry::registry::CircuitRegistry,
    types::{C, D, F},
};

#[test]
fn test() {
    let batch_circuit_config = zk_por_core::circuit_config::STANDARD_CONFIG;
    let recursive_levels = 2;
    let recursive_level_configs = vec![zk_por_core::circuit_config::STANDARD_CONFIG; 2];

    let registry =
        CircuitRegistry::<2>::init(1024, 2, batch_circuit_config, recursive_level_configs);

    let batch_circuit = registry.get_batch_circuit().0;
    let batch_proof =
        registry.get_empty_proof(&batch_circuit.verifier_only.circuit_digest).unwrap().clone();

    assert_eq!(recursive_levels, registry.get_recursive_levels());
    assert_eq!(F::ZERO, batch_proof.public_inputs[0]);
    assert_eq!(F::ZERO, batch_proof.public_inputs[1]);
    assert!(batch_circuit.verify(batch_proof).is_ok());
    let mut inner_vd_digest = batch_circuit.verifier_only.circuit_digest;

    for _ in 0..recursive_levels {
        let recursive_circuit = registry.get_recursive_circuit(&inner_vd_digest).unwrap().0;
        let recursive_empty_proof =
            registry.get_empty_proof(&recursive_circuit.verifier_only.circuit_digest).unwrap();

        assert_eq!(F::ZERO, recursive_empty_proof.public_inputs[0]);
        assert_eq!(F::ZERO, recursive_empty_proof.public_inputs[1]);
        assert!(recursive_circuit.verify(recursive_empty_proof.clone()).is_ok());

        inner_vd_digest = recursive_circuit.verifier_only.circuit_digest;
    }
    let root_circuit = registry.get_root_circuit();
    assert_eq!(inner_vd_digest, root_circuit.verifier_only.circuit_digest);
}

#[ignore] // avoid this test as this test takes a long time to run, and not necessary for the CI.
#[test]
fn test_get_root_circuit() {
    let batch_circuit_config = zk_por_core::circuit_config::STANDARD_CONFIG;
    let mut recursive_level_configs = vec![zk_por_core::circuit_config::STANDARD_CONFIG; 3];
    *recursive_level_configs.last_mut().unwrap() = zk_por_core::circuit_config::STANDARD_ZK_CONFIG;

    let batch_size = 1024;
    let token_num = 220;
    const RECURSION_BRANCHOUT_NUM: usize = 64;
    let registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        batch_circuit_config,
        recursive_level_configs,
    );
    let root_circuit = registry.get_root_circuit();

    let verifier_data = root_circuit.verifier_data().clone();

    let gate_serializer = DefaultGateSerializer;
    let verifier_data_bytes = verifier_data.to_bytes(&gate_serializer).unwrap();

    let recovered_verifier_data =
        VerifierCircuitData::<F, C, D>::from_bytes(verifier_data_bytes.clone(), &gate_serializer)
            .unwrap();
    assert_eq!(verifier_data.common, recovered_verifier_data.common);
    assert_eq!(verifier_data.verifier_only, recovered_verifier_data.verifier_only);

    println!("root_circuit_verifier_data: {:?}", hex::encode(verifier_data_bytes));
}
