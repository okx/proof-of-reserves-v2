# Test Bench
* Pingcheng Local Arm Macpro
* Date: 02/08/2024

# Config 
Plonky Config
```
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
```

# Batch Circuit
```
# be sure to change batch_size in bin/bench_batch.rs before running

cargo run --package zk-por-core --bin bench_batch --release
```
| Batch Size  | Delay (sec) |
|---|---|
| 4  | 0.016 |
| 16  | 0.022|
| 64 | 0.044|
| 256 |0.139|
| 512 |0.288|
| 1024 |0.642|

# Recursive Circuit
```
# be sure to change # of SUBPROOF in bin/bench_recursion.rs before running

cargo run --package zk-por-core --bin bench_recursion --release
```
batch_size is fixed at 1024. 

| # of SUBPROOF  | Delay (sec) |
|---|---|
| 4  | 1.34  |
| 8  | 2.80  |
| 16 | 8.65  |
| 32 | 15.58 |
| 64 | 35.16 |
| 128 | 125.36 |
