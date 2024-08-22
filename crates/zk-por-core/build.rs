use std::fs::File;
use std::io::Write;
use std::env;
use std::path::PathBuf;

fn main() {
    const U16_MAX: u32 = 1<<16;
    let array_data = (0..U16_MAX)
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest_path = out_dir.join("u16_array.rs");

    let mut file = File::create(dest_path).unwrap();
    writeln!(
        file,
        "pub const U16_TABLE: [u16; {}] = [{}];",
        U16_MAX, array_data
    ).unwrap();
}
