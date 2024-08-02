use plonky2_field::types::Field;
use zk_por_core::{circuit_registry::registry::CircuitRegistry, types::F};

#[test]
fn test() {
    let registry = CircuitRegistry::<2>::init(4, 2);

    let batch_circuit = registry.get_batch_circuit().0;
    let batch_proof = registry.get_empty_batch_circuit_proof();

    assert_eq!(F::ZERO, batch_proof.public_inputs[0]);
    assert_eq!(F::ZERO, batch_proof.public_inputs[1]);
    assert!(batch_circuit.verify(batch_proof).is_ok());

    let mut last_vd = batch_circuit.verifier_only.circuit_digest;
    let mut level = 0;
    loop {
        let recursive_ciruit = registry.get_recursive_circuit(level).0;
        let recursive_proof = registry.get_empty_recursive_circuit_proof(level);
        assert_eq!(F::ZERO, recursive_proof.public_inputs[0]);
        assert_eq!(F::ZERO, recursive_proof.public_inputs[1]);
        assert!(recursive_ciruit.verify(recursive_proof).is_ok());

        if recursive_ciruit.verifier_only.circuit_digest == last_vd {
            break;
        }
        level += 1;
        last_vd = recursive_ciruit.verifier_only.circuit_digest;
    }
}
