## to serialize
```rust

let common = root_circuit.common.clone();
let verifier_data = root_circuit.verifier_only.clone();

let gate_serializer = DefaultGateSerializer;
let gates = serde_json::to_string(&common.gates).unwrap();
println!("{:?}", gates);
let common_data_bytes = common.to_bytes(&gate_serializer).unwrap();

let common_data_hex_str = hex::encode(common_data_bytes.clone());
println!("{:?}", common_data_hex_str);

let vd_json_str = serde_json::to_string(&verifier_data).unwrap();
println!("vd_json_str: {:?}", vd_json_str);

let vd =
    VerifierData { circuit_common: common_data_hex_str, verifier_only_data: vd_json_str };

let file_path = "vd.json";
let file = File::create(file_path);
use std::io::Write;
match file {
    Ok(mut file) => {
        let json_string = serde_json::to_string_pretty(&vd).unwrap();
        file.write_all(json_string.as_bytes()).unwrap();
        println!("Data has been written to {}", file_path);
    }
    Err(e) => {
        println!("Failed to create file: {}", e);
    }
}
```