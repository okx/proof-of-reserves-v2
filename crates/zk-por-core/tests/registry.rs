use plonky2_field::types::Field;
use zk_por_core::{circuit_registry::registry::CircuitRegistry, types::F};

#[test]
fn test() {
    let batch_circuit_config = zk_por_core::circuit_config::STANDARD_CONFIG;
    let recursive_levels = 2;
    let recursive_level_configs =
        vec![zk_por_core::circuit_config::STANDARD_CONFIG; recursive_levels];

    let registry =
        CircuitRegistry::<2>::init(1024, 2, batch_circuit_config, recursive_level_configs);

    let batch_circuit = registry.get_batch_circuit().0;
    let batch_proof = registry.get_empty_batch_circuit_proof();

    assert_eq!(F::ZERO, batch_proof.public_inputs[0]);
    assert_eq!(F::ZERO, batch_proof.public_inputs[1]);
    assert!(batch_circuit.verify(batch_proof).is_ok());

    for level in 0..recursive_levels {
        let recursive_circuit = registry.get_recursive_circuit(level).unwrap().0;
        let recursive_proof = registry.get_empty_recursive_circuit_proof(level).unwrap();

        assert_eq!(F::ZERO, recursive_proof.public_inputs[0]);
        assert_eq!(F::ZERO, recursive_proof.public_inputs[1]);
        assert!(recursive_circuit.verify(recursive_proof).is_ok());
    }
}
