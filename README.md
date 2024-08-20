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

output_proof_path="global_proof.json"

cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_path}
```

- get the merkle proof
```
cargo run --release --package zk-por-cli --bin zk-por-cli get-merkle-proof --account-path account.json --output-path merkle_proof.json --cfg-path config
```

- verify
```
global_root_path="global_proof.json"

# optional. If not provided, will skip verifying the inclusion
inclusion_proof_path="merkle_proof.json"

cargo run --features zk-por-core/verifier --release --package zk-por-cli --bin zk-por-cli verify --global-proof-path ${global_root_path} --inclusion-proof-path ${inclusion_proof_path} --root 11288199779358641579,2344540219612146741,6809171731163302525,17936043556479519168
```

## cli tool
```
./target/release/zk-por-cli --help
./target/release/zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_path}
./target/release/zk-por-cli get-merkle-proof --account-path account.json --output-path merkle_proof.json --cfg-path config
```

## Code Coverage
The code test coverage report is auto generated and hosted at [codecov_report](https://okx.github.io/proof-of-reserves-v2/tarpaulin-report.html).

## Docker
```
docker build -t okx_por_v2 -f docker/Dockerfile .
```

