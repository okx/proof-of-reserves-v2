use hex::ToHex;
use plonky2::hash::hash_types::HashOut;
use rand::Rng;

#[cfg(feature = "zk-por-db")]
use plonky2::plonk::config::GenericHashOut;
#[cfg(feature = "zk-por-db")]
use std::str::FromStr;
#[cfg(feature = "zk-por-db")]
use zk_por_db::LevelDb;

use super::config::ConfigDb;

use crate::{error::PoRError, global::GLOBAL_MST, types::F};
use std::{collections::HashMap, sync::RwLock};
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct UserId(pub [u8; 32]);

impl UserId {
    pub fn rand() -> Self {
        let mut bytes: [u8; 32] = [0; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut bytes);
        Self(bytes)
    }

    pub fn to_string(&self) -> String {
        self.0.encode_hex()
    }

    pub fn from_hex_string(hex_str: String) -> Result<Self, PoRError> {
        if hex_str.len() != 64 {
            tracing::error!("User Id: {:?} is not a valid id, length is not 256 bits", hex_str);
            return Err(PoRError::InvalidParameter(hex_str));
        }

        let decode_res = hex::decode(hex_str.clone());

        if decode_res.is_err() {
            tracing::error!("User Id: {:?} is not a valid id", hex_str);
            return Err(PoRError::InvalidParameter(hex_str));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&decode_res.unwrap());

        Ok(UserId { 0: arr })
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

pub trait PoRDB: Sync + Send {
    fn add_batch_users(&mut self, batches: Vec<(UserId, u32)>);
    fn get_user_index(&self, user_id: UserId) -> Option<u32>;
    fn add_batch_gmst_nodes(&mut self, batches: Vec<(i32, HashOut<F>)>);
    fn get_gmst_node_hash(&self, node_idx: i32) -> Option<HashOut<F>>;
}

pub struct PoRLevelDBOption {
    pub user_map_dir: String,
    pub gmst_dir: String,
}

#[cfg(feature = "zk-por-db")]
pub struct PoRLevelDB {
    user_db: LevelDb<UserId>,
    gmst_db: LevelDb<i32>, // we use i32 as a key of type u32 is not provided by default in leveldb
}

#[cfg(feature = "zk-por-db")]
impl PoRLevelDB {
    pub fn new(opt: PoRLevelDBOption) -> Self {
        Self {
            user_db: LevelDb::new(&std::path::PathBuf::from_str(&opt.user_map_dir).unwrap()),
            gmst_db: LevelDb::new(&std::path::PathBuf::from_str(&opt.gmst_dir).unwrap()),
        }
    }
}

#[cfg(feature = "zk-por-db")]
impl PoRDB for PoRLevelDB {
    fn add_batch_users(&mut self, batches: Vec<(UserId, u32)>) {
        let batches = batches
            .into_iter()
            .map(|(id, idx)| (id, idx.to_be_bytes().to_vec()))
            .collect::<Vec<(UserId, Vec<u8>)>>();
        self.user_db.batch_put(batches)
    }

    fn get_user_index(&self, user_id: UserId) -> Option<u32> {
        let ret = self.user_db.get(user_id).map(|x| {
            let mut buf = [0; 4];
            buf.as_mut_slice().copy_from_slice(&x[0..4]);
            u32::from_be_bytes(buf)
        });
        ret
    }

    /// 0: the index of the gmst
    /// 1: the hash value at that index
    fn add_batch_gmst_nodes(&mut self, batches: Vec<(i32, HashOut<F>)>) {
        let batches = batches
            .into_iter()
            .map(|(id, hash)| {
                let ret = (id, hash.to_bytes());
                ret
            })
            .collect::<Vec<(i32, Vec<u8>)>>();
        self.gmst_db.batch_put(batches);
    }

    fn get_gmst_node_hash(&self, node_idx: i32) -> Option<HashOut<F>> {
        let ret = self.gmst_db.get(node_idx).map(|x| {
            let ret = HashOut::<F>::from_bytes(&x);
            ret
        });
        ret
    }
}

pub fn init_db(db_config: Option<ConfigDb>) -> Box<dyn PoRDB> {
    let database: Box<dyn PoRDB>;
    if let Some(level_db_config) = db_config {
        #[cfg(feature = "zk-por-db")]
        {
            database = Box::new(PoRLevelDB::new(PoRLevelDBOption {
                user_map_dir: level_db_config.level_db_user_path.to_string(),
                gmst_dir: level_db_config.level_db_gmst_path.to_string(),
            }));
        }

        #[cfg(not(feature = "zk-por-db"))]
        {
            _ = level_db_config;
            panic!("leveldb feature is not enabled");
        }
    } else {
        database = Box::new(PoRGMSTMemoryDB::new());
    }
    database
}
pub struct PoRMemoryDB {
    user_map: HashMap<UserId, u32>,
    gmst_map: HashMap<i32, HashOut<F>>,
}

impl PoRMemoryDB {
    pub fn new() -> Self {
        Self { user_map: HashMap::new(), gmst_map: HashMap::new() }
    }
}

impl PoRDB for RwLock<PoRMemoryDB> {
    fn add_batch_users(&mut self, batches: Vec<(UserId, u32)>) {
        for (id, idx) in batches {
            self.write().unwrap().user_map.insert(id, idx);
        }
    }

    fn get_user_index(&self, user_id: UserId) -> Option<u32> {
        self.read().unwrap().user_map.get(&user_id).map(|x| *x)
    }

    fn add_batch_gmst_nodes(&mut self, batches: Vec<(i32, HashOut<F>)>) {
        for (id, hash) in batches {
            self.write().unwrap().gmst_map.insert(id, hash);
        }
    }

    fn get_gmst_node_hash(&self, node_idx: i32) -> Option<HashOut<F>> {
        self.read().unwrap().gmst_map.get(&node_idx).map(|x| *x)
    }
}

/// PoRGMSTMemoryDB delegates the query on gmst node to the direct access of global GMST. For user_db, the query is delegated to PoRMemoryDB.
/// This is to save memory fingerprint.
pub struct PoRGMSTMemoryDB {
    user_db: RwLock<PoRMemoryDB>,
}
///
impl PoRGMSTMemoryDB {
    pub fn new() -> Self {
        Self { user_db: RwLock::new(PoRMemoryDB::new()) }
    }
}

impl PoRDB for PoRGMSTMemoryDB {
    fn add_batch_users(&mut self, batches: Vec<(UserId, u32)>) {
        self.user_db.add_batch_users(batches);
    }

    fn get_user_index(&self, user_id: UserId) -> Option<u32> {
        self.user_db.get_user_index(user_id)
    }

    #[inline(always)]
    fn add_batch_gmst_nodes(&mut self, _batches: Vec<(i32, HashOut<F>)>) {
        // do nothing as we assume GMST is already built.
        return;
    }

    fn get_gmst_node_hash(&self, node_idx: i32) -> Option<HashOut<F>> {
        GLOBAL_MST.get().unwrap().read().unwrap().inner.get(node_idx as usize).map(|x| *x)
    }
}

#[cfg(test)]
mod test {
    use plonky2::{field::types::Sample, hash::hash_types::HashOut};

    #[cfg(feature = "zk-por-db")]
    use tempdir::TempDir;

    #[cfg(feature = "zk-por-db")]
    use crate::database::{PoRLevelDB, PoRLevelDBOption};
    use crate::{
        database::{PoRDB, PoRMemoryDB, UserId},
        types::F,
    };
    use std::sync::RwLock;

    fn test_database(mut db: Box<dyn PoRDB>) {
        let batches_user = (0..4)
            .into_iter()
            .map(|i| {
                let id = UserId::rand();
                let idx = i;
                (id, idx)
            })
            .collect::<Vec<(UserId, u32)>>();
        db.add_batch_users(batches_user.clone());
        assert_eq!(db.get_user_index(batches_user[0].0), Some(0));
        assert_eq!(db.get_user_index(batches_user[3].0), Some(3));

        let batches_hash = (0..4)
            .into_iter()
            .map(|i| (i, HashOut::<F>::from_vec(vec![F::rand(), F::rand(), F::rand(), F::rand()])))
            .collect::<Vec<(i32, HashOut<F>)>>();
        db.add_batch_gmst_nodes(batches_hash.clone());

        assert_eq!(db.get_gmst_node_hash(0), Some(batches_hash[0].1));
        assert_eq!(db.get_gmst_node_hash(1), Some(batches_hash[1].1));
        assert_eq!(db.get_gmst_node_hash(2), Some(batches_hash[2].1));
        assert_eq!(db.get_gmst_node_hash(3), Some(batches_hash[3].1));
    }

    #[test]
    #[cfg(feature = "zk-por-db")]
    fn test_leveldb() {
        let tempdir_user = TempDir::new("example_user").unwrap();
        let tempdir_gmst = TempDir::new("example_gmst").unwrap();
        let db = PoRLevelDB::new(PoRLevelDBOption {
            user_map_dir: tempdir_user.path().to_string_lossy().into_owned(),
            gmst_dir: tempdir_gmst.path().to_string_lossy().into_owned(),
        });
        test_database(Box::new(db));
    }

    #[test]
    fn test_memorydb() {
        let db = PoRMemoryDB::new();
        test_database(Box::new(RwLock::new(db)));
    }
}
