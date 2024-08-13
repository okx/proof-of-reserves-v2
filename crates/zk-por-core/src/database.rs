use std::str::FromStr;

use plonky2::{hash::hash_types::HashOut, plonk::config::GenericHashOut};
use rand::Rng;
use zk_por_db::LevelDb;

use crate::types::F;

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub [u8; 32]);

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
        let dst = self.0.as_slice();
        f(&dst)
    }
}

pub struct DbOption {
    pub user_map_dir: String,
    pub gmst_dir: String,
}

pub struct DataBase {
    user_db: LevelDb<UserId>,
    gmst_db: LevelDb<i32>, // we use i32 as a key of type u32 is not provided by default in leveldb
}
impl DataBase {
    pub fn new(opt: DbOption) -> Self {
        Self {
            user_db: LevelDb::new(&std::path::PathBuf::from_str(&opt.user_map_dir).unwrap()),
            gmst_db: LevelDb::new(&std::path::PathBuf::from_str(&opt.gmst_dir).unwrap()),
        }
    }

    pub fn add_batch_users(&mut self, batches: Vec<(UserId, u32)>) {
        let batches = batches
            .into_iter()
            .map(|(id, idx)| (id, idx.to_be_bytes().to_vec()))
            .collect::<Vec<(UserId, Vec<u8>)>>();
        self.user_db.batch_put(batches)
    }

    pub fn get_user_index(&self, user_id: UserId) -> Option<u32> {
        let ret = self.user_db.get(user_id).map(|x| {
            let mut buf = [0; 4];
            buf.as_mut_slice().copy_from_slice(&x[0..4]);
            u32::from_be_bytes(buf)
        });
        ret
    }

    /// 0: the index of the gmst
    /// 1: the hash value at that index
    pub fn add_batch_gmst_nodes(&mut self, batches: Vec<(i32, HashOut<F>)>) {
        let batches = batches
            .into_iter()
            .map(|(id, hash)| {
                let ret = (id, hash.to_bytes());
                ret
            })
            .collect::<Vec<(i32, Vec<u8>)>>();
        self.gmst_db.batch_put(batches);
    }

    pub fn get_gmst_node_hash(&self, node_idx: i32) -> Option<HashOut<F>> {
        let ret = self.gmst_db.get(node_idx).map(|x| {
            let ret = HashOut::<F>::from_bytes(&x);
            ret
        });
        ret
    }
}

#[cfg(test)]
mod test {
    use tempdir::TempDir;

    use crate::database::{DataBase, DbOption, UserId};

    #[test]
    fn test_user() {
        let tempdir_user = TempDir::new("example_user").unwrap();
        let tempdir_gmst = TempDir::new("example_gmst").unwrap();
        let mut db = DataBase::new(DbOption {
            user_map_dir: tempdir_user.path().to_string_lossy().into_owned(),
            gmst_dir: tempdir_gmst.path().to_string_lossy().into_owned(),
        });

        let batches = (0..4)
            .into_iter()
            .map(|i| {
                let id = UserId::rand();
                let idx = i;
                (id, idx)
            })
            .collect::<Vec<(UserId, u32)>>();
        db.add_batch_users(batches.clone());
        assert_eq!(db.get_user_index(batches[0].0), Some(0));
        assert_eq!(db.get_user_index(batches[3].0), Some(3));
    }
}
