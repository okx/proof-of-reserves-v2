rm -rf release
mkdir -p release/{config,sample_data}

cargo build --release
mv target/release/zk-por-cli release/zk-por-prover
cp release/zk-por-prover release/zk-por-checker

cargo build --features zk-por-core/verifier --release
mv target/release/zk-por-cli release/zk_STARK_Validator_v2

mkdir -p release/config
sed 's|/opt/data/zkpor/users/|./sample_data/|g' config/default.toml > release/config/default.toml

mkdir -p release/sample_data
cp -r test-data/batch0.json release/sample_data
cp docs/release.md release/README.md

tar -cvf zk-por-mac-arm.tar ./release/