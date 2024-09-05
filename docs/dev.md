# release procedule
## for linux
ssh in a linux machine
```
version=release/v0.1.0
git checkout ${version}
cargo build --release
mv target/release/zk-por-cli target/release/zk-por-prover
cargo build --features zk-por-core/verifier --release
mv target/release/zk-por-cli target/release/zk-por-verifier
tar -cvf zk-por-x86_64-unknown-linux-gnu.tar target/release/zk-por-prover target/release/zk-por-verifier
```