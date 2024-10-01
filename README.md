![Coverage](https://raw.githubusercontent.com/okx/proof-of-reserves-v2/gh-pages/coverage-badge.svg)

# proof-of-reserves-v2

## Background

OKX launches [Proof of Reserves (PoR)](https://www.okx.com/proof-of-reserves) to improve the security and transparency
of users' assets. These tools will allow users to independently audit OKX's Proof of Reserves and verify OKX's reserves
exceed the exchange's known liabilities to users, in order to confirm the solvency of OKX.

## Technical Specs
The technical details can be found in the [technical specs doc](./docs/technical_spec.md).

## Liabilities
OKX's PoR uses Zero-knowledge (ZK) Merkle Sum Tree technology to allow each user to independently review OKX's digital asset reserve on the
basis of protecting user's privacy. We use Plonky2 to build the proofs of users' assets using a Merkle Sum Tree. A detailed documentation of the technical solution can be found in the [technical specs doc](./docs/technical_spec.md).

## How to Run
- generate test data
```
file_num=10
per_file_account_num=131072 # multiple of 1024, the batch size

# test data will be generated to ./test-data/user-data
python3 scripts/gen_test_data.py ${file_num} ${per_file_account_num}
```

- prove
```
cfg_dir_path="config"

cp ${cfg_dir_path}/default.toml ${cfg_dir_path}/local.toml

# edit local.toml such that the field "user_data_path" to "test-data/user-data"
sed -i '' 's|/opt/data/zkpor/users/|test-data/user-data|g' config/local.toml

output_proof_dir_path="./test-data/proof"

cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
```

With GPU support, run:
```
export NUM_OF_GPUS=1
cargo run --release --features=cuda --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
```
This should be 40-50% faster than running on the CPU. Note: you need at least 24 GB of GPU RAM to run the code with the current parameters. This was tested on an RTX 4090 GPU.


- verify global proof
```
global_proof_path="./test-data/proof/sum_proof_data.json"

# on CPU-only
cargo run --features zk-por-core/verifier --release --package zk-por-cli --bin zk-por-cli verify-global --proof-path ${global_proof_path}

# on GPU
cargo run --features=cuda,zk-por-core/verifier --release --package zk-por-cli --bin zk-por-cli verify-global --proof-path ${global_proof_path}
```

- verify user proof
```
global_proof_path="./test-data/proof/sum_proof_data.json"
# to verify all accounts
user_proof_path_pattern="./test-data/proof/user_proofs/*.json"

# to verify one account with ${accountID}
# user_proof_path_pattern="./test-data/proof/user_proofs/${accountID}.json"

# on CPU-only
cargo run --features zk-por-core/verifier --release --package zk-por-cli --bin zk-por-cli verify-user --global-proof-path ${global_proof_path} --user-proof-path-pattern ${user_proof_path_pattern}

# on GPU
cargo run --features=cuda,zk-por-core/verifier --release --package zk-por-cli --bin zk-por-cli verify-user --global-proof-path ${global_proof_path} --user-proof-path-pattern ${user_proof_path_pattern}
```

## cli tool
```
./target/release/zk-por-cli --help
```

## Code Coverage
The code test coverage report is auto generated and hosted at [codecov_report](https://okx.github.io/proof-of-reserves-v2/tarpaulin-report.html).

## Docker
```
docker build -t okx_por_v2 -f docker/Dockerfile .
```

