extern crate leveldb;
extern crate tempdir;

use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, WriteOptions, ReadOptions};
use tempdir::TempDir;


fn main() {
    // Create a temporary directory to store the database
    // let tempdir = TempDir::new("example").unwrap();
    let db_path = std::path::Path::new("./my_permanent_leveldb");

    // Create options for opening the database
    let mut options = Options::new();
    options.create_if_missing = true;

    // Open the database
    let database = match Database::<i32>::open(db_path, options) {
        Ok(db) => db,
        Err(e) => {
            println!("Failed to open database: {:?}", e);
            return;
        }
    };

    // Define the write options
    let write_opts = WriteOptions::new();
    // Put some data into the database
    let key = 1;
    let value = b"value1";
    match database.put(write_opts, key, value) {
        Ok(_) => println!("Successfully wrote key-value pair"),
        Err(e) => println!("Failed to write key-value pair: {:?}", e),
    }

    // Define the read options
    let read_opts = ReadOptions::new();
    // Read the data back
    match database.get(read_opts, key) {
        Ok(Some(value)) => println!("Read value: {:?}", String::from_utf8(value).unwrap()),
        Ok(None) => println!("Value not found"),
        Err(e) => println!("Failed to read value: {:?}", e),
    }

    // Deleting a key
    match database.delete(write_opts, key) {
        Ok(_) => println!("Successfully deleted key"),
        Err(e) => println!("Failed to delete key: {:?}", e),
    }

    // Close the database automatically by dropping it
}