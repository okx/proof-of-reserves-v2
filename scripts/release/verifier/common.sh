function build_and_package() {
	TARGET=$1
    rustup target add $TARGET
    VERSION=$2
    # below cargo build will build $COMMIT_HASH into binary. 
    export COMMIT_HASH=$(git rev-parse --short HEAD)
    
    RUSTFLAGS="-C target-feature=+crt-static" cargo build --features zk-por-core/verifier --release --target ${TARGET} --package zk-por-cli --bin zk-por-cli 

    VALIDATOR_BIN="./zk_STARK_Validator_V2"
    if [ -f target/${TARGET}/release/zk-por-cli ]; then
        mv target/${TARGET}/release/zk-por-cli $VALIDATOR_BIN
    elif [ -f target/${TARGET}/release/zk-por-cli.exe ]; then
        mv target/${TARGET}/release/zk-por-cli.exe $VALIDATOR_BIN
    else
        echo "zk-por-cli binary does not exist."
        return 1
    fi 

    zip ./zk_STARK_Validator_V2_${TARGET}_${VERSION}.zip $VALIDATOR_BIN
    rm $VALIDATOR_BIN
    unset COMMIT_HASH
}