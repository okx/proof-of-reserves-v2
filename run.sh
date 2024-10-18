#!/bin/bash

cfg_dir_path="config"
output_proof_dir_path="./test-data/proof"

tstamp=`date +%Y-%m-%d-%H-%M-%S`
logfile="log-run-${tstamp}.txt"

# CPU-only, no AVX
rm -rf test-data/proof
cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path} > ${logfile} 2>&1

# CPU-only, with AVX2
rm -rf test-data/proof
RUSTFLAGS="-C target-feature=+avx2" cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path} >> ${logfile} 2>&1

# CPU-only, with AVX2 and AVX-512
rm -rf test-data/proof
RUSTFLAGS="-C target-feature=+avx2,+avx512dq" cargo run --release --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path} >> ${logfile} 2>&1

# CPU+GPU
rm -rf test-data/proof
export NUM_OF_GPUS=1
cargo run --release --features=cuda --package zk-por-cli --bin zk-por-cli prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path} >> ${logfile} 2>&1