function build_and_package() {
    mkdir -p validator/bin
	TARGET=$1
    VERSION=$2
    TMR_DIR=""
    RUSTFLAGS="-C target-feature=+crt-static" cargo build --features zk-por-core/verifier --release --target ${TARGET} --package zk-por-cli --bin zk-por-cli 

    mv target/${TARGET}/release/zk-por-cli ./zk_STARK_Validator_V2_${TARGET}_${VERSION}
    zip ./zk_STARK_Validator_V2_${TARGET}_${VERSION}.zip ./zk_STARK_Validator_V2_${TARGET}_${VERSION}
    rm ./zk_STARK_Validator_V2_${TARGET}_${VERSION}
    mv ./zk_STARK_Validator_V2_${TARGET}_${VERSION}.zip validator/bin
}