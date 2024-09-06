rm -rf release
mkdir -p release/{config,sample_data}

cargo build --release
mv target/release/zk-por-cli release/zk-por-prover
cargo build --features zk-por-core/verifier --release
mv target/release/zk-por-cli release/zk-por-verifier

mkdir -p release/config
cp config/default.toml release/config/default.toml
sed -i '' 's|/opt/data/zkpor/users/|./sample_data/|g' release/config/default.toml

mkdir -p release/sample_data
cp -r test-data/batch0.json release/sample_data
cp doc/release.md release/README.md

tar -cvf zk-por.tar ./release/