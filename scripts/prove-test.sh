#!/bin/bash

cfg_dir_path="config"

cp ${cfg_dir_path}/default.toml ${cfg_dir_path}/local.toml

# edit local.toml such that the field "user_data_path" to "test-data/user-data"
sed -i 's|/opt/data/zkpor/users/|test-data/user-data|g' config/local.toml

output_proof_dir_path="./test-data/proof"
rm -rf ${output_proof_dir_path}

# cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
export FORCE_SINGLE_GPU=true
export NUM_OF_GPUS=1
# RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+avx512dq" cargo run --release --features=cuda --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
# cargo run --release --features=cuda --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}

# CPU (no vectorization)
cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}

# CPU (with AVX512)
# RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+avx512dq" cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
