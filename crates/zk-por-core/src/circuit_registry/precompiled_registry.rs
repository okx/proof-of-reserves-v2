use crate::types::{C, D, F};
use core::panic;
use once_cell::sync::Lazy;
use plonky2::{
    plonk::circuit_data::VerifierCircuitData, util::serialization::DefaultGateSerializer,
};
use std::collections::{BTreeSet, HashMap, HashSet};

// Refer to README on how to generate the hex string for the root circuit.
static LATEST_ROOT_CIRCUIT_HEX: &str = "0100000000000000e3370d1646daebec7fa045ddf1cc918706777cb88875ea173623eff57b773bc68f62cdf279612bd8c095eb7bad5feaf5209df02981e0b88ef5b178862e00694ba296ce1215562eabcac32a3b9a5aae1dcf6422a739a392ffc1b87f102bdf523d8700000000000000500000000000000002000000000000006400000000000000020000000000000008000000000000000101030000000000000001000000000000001c00000000000000100000000104000000000000000500000000000000030000000000000001000000000000001c00000000000000100000000104000000000000000500000000000000040000000000000004000000000000000400000000000000040000000000000004000000000000001300000000000000010e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000001000000000000000100000000000000010000000000000001000000000000000200000000000000020000000000000003000000000000000000000000000000070000000000000007000000000000000c000000000000000c000000000000000e0000000000000008000000000000007b0000000000000005000000000000000600000000000000500000000000000001000000000000000700000000000000310000000000000057010000000000006109000000000000a74100000000000091cb010000000000f7900c0000000000c1f657000000000047bf670200000000f13ad61000000000979cdb75000000002148013903000000e7f8088f1600000051ce3ee99d00000037a4b76051040000817d05a5391e0000876e268393d30000b1050d9608c90500d7275b1a3c7f2800e1167eb8a47a1b0127a0720b815ac007116122508779423676a7f030b452d17b37949456f042b9627f0d105e94d410b3755e709212d075e52d95120188b038463a148207b9d38ceb908d8e3415cad970eddee56f9786f4157b18490f24aeaf9959abff6a00c3cd336eaffdec0355a06a00ccef7a1d5362eafa938e5cd445b068d40be687d0e8d1dcc6524ab7b95dbd096a43080314902d44e5d739158df03edd3de79494e193b80cab5212102b0b0c59ab4280702f4e546faad281134f234e0ba6c28c8829f7224f8852d9bb24c2f429b741f122024fb12500cc98f40f29d90700942db06f1ff036ff0b3fd10edc9080f653b9b86a04f683b74b110dee1eba9bfd11795b86d81642ea7d4f80adeb9fce61712c82c3715fa6a319378f5c1c9c8c72b381ea8ac644d819e88b69d16de1e9a958d3e2bf002a659d6cc733410526c446f8736acd240a5de8c92be99f01478b55853260620bf1ce4ea561a1b54f97a81e85ab69fb2d239ccea3b0e3e341f644a17ad4393ccdbbe2615acf94ab9c2233a678ab11b248f265884cb07be0fc9fc9b317d26128ea5f83e2a5beac1d679f972a8936b9d3d15b2525c07d10cbbc8205034170738d29932614c71128df22060e8c717c181af42a62d21a67abb8ac2cafbabbd1af10b938ca1122bcce790f8d8709000000000000000e00000000000000090000000b0000000c000000020000003f000000000000000400000004000000000000000e0000000100000000000000140000000000000000000000000000000f0000002000000000000000100000002b00000000000000010000000a00000000000000000000001400000000000000080000000d000000000000000500000042000000000000000e0000000400000000000000040000000000000002000000000000000d000000";

// key: a number of round numbers, value: the hex string of the root circuit verifier data.
static PREV_ROOT_CIRCUITS_HEX: Lazy<HashMap<BTreeSet<usize>, String>> = Lazy::new(|| {
    let ret: HashMap<BTreeSet<usize>, String> = HashMap::new();
    // later add previous root circuits here.

    // check no duplicates of round numbers in the keys
    let mut existing_round_nums = HashSet::new();
    ret.iter().for_each(|(circuit_round_nums, _)| {
        circuit_round_nums.iter().for_each(|&round_num| {
            if existing_round_nums.contains(&round_num) {
                panic!("duplicate round number found in the keys of PREV_ROOT_CIRCUITS_HEX");
            }
            existing_round_nums.insert(round_num);
        });
    });
    ret
});

pub fn get_verifier_for_round(round_num: usize) -> VerifierCircuitData<F, C, D> {
    let circuit_hex =
        get_circuit_hex_str(LATEST_ROOT_CIRCUIT_HEX, &PREV_ROOT_CIRCUITS_HEX, round_num);
    let root_circuit_verifier_data_bytes = hex::decode(circuit_hex).expect(
        format!("fail to decode root circuit verifier data hex string for round {}", round_num)
            .as_str(),
    );

    let root_circuit_verifier_data = VerifierCircuitData::<F, C, D>::from_bytes(
        root_circuit_verifier_data_bytes,
        &DefaultGateSerializer,
    )
    .expect(format!("fail to parse root circuit verifier data for round {}", round_num).as_str());

    root_circuit_verifier_data
}

fn get_circuit_hex_str(
    latest_circuit_hex: &str,
    prev_circuits_hex: &HashMap<BTreeSet<usize>, String>,
    round_num: usize,
) -> String {
    let mut circuit_hex = latest_circuit_hex.to_owned();
    if let Some(prev_circuit_hex) =
        (prev_circuits_hex.iter().find(|(k, _)| k.contains(&round_num))).map(|(_, v)| v)
    {
        tracing::info!("found previous circuit for round {}", round_num);
        circuit_hex = prev_circuit_hex.clone();
    }
    circuit_hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    #[test]
    fn test_get_circuit_hex_str_with_prev_circuit() {
        let latest_circuit_hex = "latest_hex";
        let mut prev_circuits_hex = HashMap::new();

        let round_num = 1;
        let result = get_circuit_hex_str(latest_circuit_hex, &prev_circuits_hex, round_num);

        assert_eq!(result, latest_circuit_hex);

        let mut round_set = BTreeSet::new();
        round_set.insert(round_num);
        prev_circuits_hex.insert(round_set, "prev_hex_for_round_1".to_string());

        let result = get_circuit_hex_str(latest_circuit_hex, &prev_circuits_hex, round_num);

        assert_eq!(result, "prev_hex_for_round_1");
    }
}
