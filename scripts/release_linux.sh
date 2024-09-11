rm -rf release
mkdir -p release/{config,sample_data}

RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-gnu
mv target/x86_64-unknown-linux-gnu/release/zk-por-cli release/zk-por-prover
RUSTFLAGS="-C target-feature=+crt-static" cargo build --features zk-por-core/verifier --release --target x86_64-unknown-linux-gnu
mv target/x86_64-unknown-linux-gnu/release/zk-por-cli release/zk-por-verifier

mkdir -p release/config
sed 's|/opt/data/zkpor/users/|./sample_data/|g' config/default.toml > release/config/default.toml

mkdir -p release/sample_data
cp -r test-data/batch0.json release/sample_data
cp docs/release.md release/README.md

tar -cvf zk-por-linux.tar ./release/