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

cargo bench --package zk-por-core -- batch_circuit
```
## Setting
asset_num = 200
## Result
| Batch Size  | Delay (sec) |
|---|---|
| 16  | 0.031|
| 64 | 0.087|
| 256 |0.300|
| 512 |0.712|
| 1024 |1.254|

# Recursive Circuit
```
# be sure to change # of SUBPROOF in bin/bench_recursion.rs before running

cargo bench --package zk-por-core -- recursive_circuit
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

# E2E
## Cmd
```
file_num=25
per_file_batch_num=40
cargo run --release --package zk-por-core --bin bench_prover ${file_num} ${per_file_batch_num}
```

## Setting
* batch_size = 1024
* recursion_factor = 64
* proving_threads = 4
* num_accounts = file_num * per_file_batch_num * batch_size

## Test Bench and Date
* Pingcheng Local Arm Macpro
* Date: 08/08/2024

## Result
building circuit: 3min. 

| # of accounts  | Batch Proving Delay | E2E Delay |
|---|---|---|
| 10^3 | 1.52s  | 123s|
| 10^4 | 10s | 121s |
| 10^5 |104s| 256s| 
| 10^6 | ||

NOTE:
* E2E Delay excludes the circuit pre-building time. 
* Batch proving delay measures the time to prove account batches, without recursive layers. 