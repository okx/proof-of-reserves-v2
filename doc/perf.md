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
## Effects of Batch Size
```
asset_num=200
parallism=1

cargo bench --package zk-por-core -- batch_circuit_\\d+_asset_num_${asset_num}_parallism_${parallism}$
```
| Batch Size  | Delay (sec) |
|---|---|
| 16  | 0.031|
| 64 | 0.087|
| 256 |0.300|
| 512 |0.712|
| 1024 |1.254|

## Effects of Asset Number
```
parallism=1
batch_size=1024

cargo bench --package zk-por-core -- batch_circuit_${batch_size}_asset_num_\\d+_parallism_${parallism}
```
| Number of Assets  | Delay (sec) |
|---|---|
| 4 | 0.618|
| 20 | 0.668 | 
| 50 | 0.624 |
| 100 | 1.22 | 
| 200 | 1.31 | 

## Effects of Parallism
### Batch Size = 1024
Number of concurrent threads, each proving a batch. 
```
asset_num=200
batch_size=1024

cargo bench --package zk-por-core -- batch_circuit_${batch_size}_asset_num_${asset_num}_parallism_\\d+$
```

| Parallism  | Delay (sec) |
|---|---|
| 1 | 1.42|
| 2 | 2.37|
| 4 | 4.32|
| 8 | 7.77|
| 16 | 16.95|
| 32 | 32.627|

### Batch Size = 16
Number of concurrent threads, each proving a batch. 
```
asset_num=200
batch_size=16

cargo bench --package zk-por-core -- batch_circuit_${batch_size}_asset_num_${asset_num}_parallism_\\d+$
```

| Parallism  | Delay (sec) |
|---|---|
| 1 | 0.028|
| 2 | 0.046|
| 4 | 0.084|
| 8 | 0.163|
| 16 | 0.321|
| 32 | 0.641|

# Recursive Circuit
## Effects of Subproof Size
```
parallism=1
cargo bench --package zk-por-core -- recursive_circuit_\\d+_parallism_${parallism}$
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

## Effects of Parallism
```
subproof_num=64
cargo bench --package zk-por-core -- recursive_circuit_${subproof_num}_parallism_\\d+$

```
batch_size is fixed at 1024. 

| parallism   | Delay (sec) |
|---|---|


# E2E
## Cmd
## Setting
* batch_size = 1024
* recursion_factor = 64
* batch_proving_threads = 4
* recursive_proving_threads = 2
* num_accounts = file_num * per_file_batch_num * batch_size

## Test Bench and Date
* Pingcheng Local Arm Macpro
* Date: 08/08/2024

## Cmd
```
account_num=1024
cargo run --release --package zk-por-core --bin prover -- --bench ${account_num}
```

## Result
| # of accounts  | Batch Proving Delay | E2E Delay |
|---|---|---|
| 10^3 | 1.52s  | 123s|
| 10^4 | 10s | 121s |
| 10^5 |104s| 256s| 
| 10^6 |16min |28min|
| 10^7 | 160min (expected) | 250min (expected) | 

NOTE:
* E2E Delay excludes the circuit pre-building time. 
* Batch proving delay measures the time to prove account batches, without recursive proving. 
* No performance changes when batch_proving_threads increased to 16, as CPU already saturated (utilization rate at 1000%)
* OOM when recursive_proving_threads=4. 
* The above figure excludes the time to prebuild circuit and precompute empty proofs, which takes 3 minutes. 

