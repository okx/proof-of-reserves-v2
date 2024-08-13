extern crate leveldb;
extern crate tempdir;

use leveldb::{
    database::{
        batch::{Batch, Writebatch},
        Database,
    },
    kv::KV,
    options::{Options, ReadOptions, WriteOptions},
};
use tracing::warn;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct UserId([u8; 32]);

impl UserId {
    pub fn rand() -> Self {
        let mut bytes: [u8; 32] = [0; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut bytes);
        Self(bytes)
    }
}

impl db_key::Key for UserId {
    fn from_u8(key: &[u8]) -> UserId {
        assert!(key.len() == 32);
        let mut output: [u8; 32] = [0; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(key.as_ptr(), output.as_mut_ptr(), 32);
        }
        UserId(output)
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        let mut dst = self.0.as_slice();
        f(&dst)
    }
}

pub struct LevelDb<K: db_key::Key> {
    db: Database<K>,
}

impl<K: db_key::Key> LevelDb<K> {
    pub fn new(db_path: &std::path::Path) -> Self {
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

    /// input is a vector of (k,v) tuple
    pub fn batch_put(&self, batches: Vec<(K, &[u8])>) {
        let mut batch = Writebatch::<K>::new();
        batches.into_iter().for_each(|(k, v)| {
            batch.put(k, v);
        });

        match self.db.write(WriteOptions::new(), &batch) {
            Ok(_) => (),
            Err(e) => panic!("Batch write failed: {}", e),
        }
    }

    pub fn get(&self, key: K) -> Option<Vec<u8>> {
        let read_opts = ReadOptions::new();
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
    use std::path::Path;

    use tempdir::TempDir;

    use crate::{LevelDb, UserId};
    use leveldb::{
        database::{
            batch::{Batch, Writebatch},
            Database,
        },
        kv::KV,
        options::{Options, ReadOptions, WriteOptions},
    };

    #[test]
    fn test_db_i32() {
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

    #[test]
    fn test_db_bytes() {
        let tempdir = TempDir::new("example").unwrap();
        let db = LevelDb::<UserId>::new(tempdir.path());
        let key = UserId::rand();
        db.put(key.clone(), b"hello");
        // db.put(2, b"world");
        let ret = db.get(key).unwrap();

        assert_eq!(ret, b"hello");
        // db.delete(1);
        // let ret = db.get(1);
        // assert_eq!(ret, None);
        // db.delete(2);
    }
}
