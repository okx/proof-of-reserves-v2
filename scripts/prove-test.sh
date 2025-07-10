#!/bin/bash

cfg_dir_path="config"
output_proof_dir_path="./test-data/proof"

cp ${cfg_dir_path}/default.toml ${cfg_dir_path}/local.toml
sed -i 's|/opt/data/zkpor/users/|test-data/user-data|g' config/local.toml
rm -rf ${output_proof_dir_path}

FEATURES_CPU="--features=no_cuda,async"
FEATURES_GPU="--features=cuda,async"

export FORCE_SINGLE_GPU=true
export NUM_OF_GPUS=1

# GPU (no vectorization)
# cargo run --release ${FEATURES_GPU} --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}

# GPU (with AVX512)
# RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+avx512dq" cargo run --release ${FEATURES_GPU} --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}

# CPU (no vectorization)
# cargo run --release ${FEATURES_CPU} --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}

# CPU (with AVX512)
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+avx512dq" cargo run --release ${FEATURES_CPU} --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
