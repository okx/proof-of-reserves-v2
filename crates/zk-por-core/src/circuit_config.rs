use plonky2::{
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    plonk::circuit_data::CircuitConfig,
};

pub const STANDARD_CONFIG: CircuitConfig = CircuitConfig {
    num_wires: 135,
    num_routed_wires: 80,
    num_constants: 2,
    use_base_arithmetic_gate: true,
    security_bits: 100,
    num_challenges: 2,
    zero_knowledge: false,
    max_quotient_degree_factor: 8,
    fri_config: FriConfig {
        rate_bits: 3,
        cap_height: 1,
        proof_of_work_bits: 16,
        reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
        num_query_rounds: 28,
    },
};

// A high-rate recursive proof, designed to be verifiable with fewer routed wires.
pub const HIGH_RATE_CONFIG: CircuitConfig = CircuitConfig {
    num_wires: 135,
    num_routed_wires: 80,
    num_constants: 2,
    use_base_arithmetic_gate: true,
    security_bits: 100,
    num_challenges: 2,
    zero_knowledge: false,
    max_quotient_degree_factor: 8,
    fri_config: FriConfig {
        rate_bits: 7,
        cap_height: 1,
        proof_of_work_bits: 16,
        reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
        num_query_rounds: 12,
    },
};

pub const STANDARD_ZK_CONFIG: CircuitConfig = CircuitConfig {
    num_wires: 135,
    num_routed_wires: 80,
    num_constants: 2,
    use_base_arithmetic_gate: true,
    security_bits: 100,
    num_challenges: 2,
    zero_knowledge: true,
    max_quotient_degree_factor: 8,
    fri_config: FriConfig {
        rate_bits: 3,
        cap_height: 1,
        proof_of_work_bits: 16,
        reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
        num_query_rounds: 28,
    },
};

pub fn get_recursive_circuit_configs<const RECURSION_BRANCHOUT_NUM: usize>(
    batch_num: usize,
) -> Vec<CircuitConfig> {
    let level = (batch_num as f64).log(RECURSION_BRANCHOUT_NUM as f64).ceil() as usize;

    let mut configs = vec![STANDARD_CONFIG; level];

    if let Some(last) = configs.last_mut() {
        *last = STANDARD_ZK_CONFIG; // Change the last element to 0
    } else {
        configs.push(STANDARD_ZK_CONFIG); // Add 0 if the vec is empty
    }
    configs
}

#[cfg(test)]
pub mod test {
    use crate::circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG};

    use super::get_recursive_circuit_configs;

    #[test]
    pub fn test_get_recursive_circuit_config(){
        let batch_num = 1;
        let cfgs = get_recursive_circuit_configs::<64>(batch_num);
        assert_eq!(vec![STANDARD_ZK_CONFIG], cfgs);

        let batch_num = 66;
        let cfgs = get_recursive_circuit_configs::<64>(batch_num);
        assert_eq!(vec![STANDARD_CONFIG, STANDARD_ZK_CONFIG], cfgs);
    }
}
