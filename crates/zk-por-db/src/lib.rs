extern crate leveldb;
extern crate tempdir;

use leveldb::{
    database::Database,
    kv::KV,
    options::{Options, ReadOptions, WriteOptions},
};
use tracing::warn;

pub struct LevelDb<K: db_key::Key> {
    db: Database<K>,
}

impl<K: db_key::Key> LevelDb<K> {
    pub fn new(db_path: &std::path::Path) -> Self {
        // Create options for opening the database
        let mut options = Options::new();
        options.create_if_missing = true;

        let database = match Database::<K>::open(db_path, options) {
            Ok(db) => db,
            Err(e) => {
                panic!("Failed to open database: {:?}", e);
            }
        };
        Self { db: database }
    }

    pub fn put(&self, key: K, val: &[u8]) {
        let write_opts = WriteOptions::new();

        match self.db.put(write_opts, key, val) {
            Ok(_) => (),
            Err(e) => panic!("Failed to write key-value pair: {:?}", e),
        }
    }

    pub fn get(&self, key: K) -> Option<Vec<u8>> {
        // Define the read options
        let read_opts = ReadOptions::new();
        // Read the data back
        match self.db.get(read_opts, key) {
            Ok(val) => val,
            Err(e) => {
                warn!("Failed to read value: {:?}", e);
                None
            }
        }
    }

    pub fn delete(&self, key: K) {
        let write_opts = WriteOptions::new();
        match self.db.delete(write_opts, key) {
            Ok(_) => (),
            Err(e) => warn!("Failed to delete key: {:?}", e),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate tempdir;
    use tempdir::TempDir;

    use crate::LevelDb;

    #[test]
    fn test_db() {
        let tempdir = TempDir::new("example").unwrap();
        let db = LevelDb::<i32>::new(tempdir.path());
        db.put(1, b"hello");
        db.put(2, b"world");
        let ret = db.get(1).unwrap();

        assert_eq!(ret, b"hello");
        db.delete(1);
        let ret = db.get(1);
        assert_eq!(ret, None);
        db.delete(2);
    }
}
