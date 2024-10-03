function build_and_package() {
	TARGET=$1
    rustup target add $TARGET
	RELEASE_TMP_DIR="./release"
    rm -rf ${RELEASE_TMP_DIR}
    mkdir -p ${RELEASE_TMP_DIR}/{config,sample_data}
    
    # below cargo build will build $COMMIT_HASH into binary. 
    export COMMIT_HASH=$(git rev-parse --short HEAD)

    RUSTFLAGS="-C target-feature=+crt-static" cargo build --features zk-por-core/zk-por-db --release --target ${TARGET} --package zk-por-cli --bin zk-por-cli 

    mv target/${TARGET}/release/zk-por-cli ${RELEASE_TMP_DIR}/zk-por-prover

    RUSTFLAGS="-C target-feature=+crt-static" cargo build --features zk-por-core/verifier --release --target ${TARGET} --package zk-por-cli --bin zk-por-cli 

    mv target/${TARGET}/release/zk-por-cli ${RELEASE_TMP_DIR}/zk_STARK_Validator_v2

	cp ${RELEASE_TMP_DIR}/zk_STARK_Validator_v2 ${RELEASE_TMP_DIR}/zk-por-checker

    sed 's|/opt/data/zkpor/users/|./sample_data/|g' config/default.toml > ${RELEASE_TMP_DIR}/config/default.toml

    cp -r test-data/batch0.json ${RELEASE_TMP_DIR}/sample_data
    cp docs/release.md ${RELEASE_TMP_DIR}/README.md

    tar -cvf zk-por-${TARGET}.tar ${RELEASE_TMP_DIR}
	rm -rf ${RELEASE_TMP_DIR}
    unset COMMIT_HASH
}