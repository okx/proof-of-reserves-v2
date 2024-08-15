use std::{collections::BTreeMap, fs::File, io::BufReader};

use super::account::{gen_accounts_with_random_data, Account};
use crate::types::F;
use plonky2_field::types::Field;
use serde_json::Value;
use std::{
    fs,
    ops::Div,
    path::{Path, PathBuf},
};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct FilesCfg {
    pub dir: PathBuf,
    pub batch_size: usize,
    pub num_of_tokens: usize,
}

pub trait AccountParser {
    fn read_n_accounts(&mut self, offset: usize, n: usize) -> Vec<Account>;
    fn total_num_of_users(&self) -> usize;
}

#[derive(Debug)]
pub struct FilesParser {
    pub cfg: FilesCfg,
    pub total_num_of_batches: usize,
    pub total_num_of_users: usize,
    num_of_docs: usize,
    num_of_batches_per_doc: usize,
    file_idx: usize,
    offset: usize, // an offset to the buffered_accounts
    buffered_accounts: Vec<Account>,
    last_doc_accounts: Vec<Account>, // we keep this avoiding double loading of last doc
    docs: Vec<PathBuf>,
}

impl FilesParser {
    pub fn new(cfg: FilesCfg) -> Self {
        let mut parser = Self {
            buffered_accounts: vec![],
            last_doc_accounts: vec![],
            file_idx: 0,
            offset: 0,
            num_of_docs: 0,
            cfg,
            num_of_batches_per_doc: 0,
            total_num_of_users: 0,
            total_num_of_batches: 0,
            docs: vec![],
        };
        let user_data_path = std::path::Path::new(&parser.cfg.dir);
        if !user_data_path.exists() {
            panic!("dir: {:?} does not exist", user_data_path);
        }

        let json_files = list_json_files(user_data_path);

        match json_files {
            Ok(docs) => {
                info!("files: {:?}", docs);
                let doc_len = docs.len();
                parser.docs = docs;
                parser.num_of_docs = doc_len;
                if doc_len < 1 {
                    panic!("no json files under the folder: {:?}", user_data_path);
                }
                let first_doc_accounts =
                    read_json_into_accounts_vec(parser.docs[0].to_str().unwrap());
                let first_doc_accounts_len = first_doc_accounts.len();

                if doc_len > 1 {
                    assert_eq!(first_doc_accounts_len % parser.cfg.batch_size, 0);
                }
                parser.num_of_batches_per_doc = first_doc_accounts_len.div(parser.cfg.batch_size);
                parser.buffered_accounts = first_doc_accounts;
                parser.file_idx = 0;

                if doc_len > 1 {
                    let last_doc_accounts =
                        read_json_into_accounts_vec(parser.docs[doc_len - 1].to_str().unwrap());
                    assert!(last_doc_accounts.len() <= first_doc_accounts_len);
                    parser.last_doc_accounts = last_doc_accounts;
                }

                let total_num_of_users =
                    (doc_len - 1) * first_doc_accounts_len + parser.last_doc_accounts.len();

                let num_of_batches = total_num_of_users.div_ceil(parser.cfg.batch_size);
                parser.total_num_of_users = total_num_of_users;
                parser.total_num_of_batches = num_of_batches;
            }
            Err(e) => panic!("list json files err: {:?}", e),
        }

        parser
    }

    pub fn log_state(&self) {
        debug!("cfg: {:?},\n num_of_files: {:?},\n num_of_batches_per_doc: {:?},\n file_idx: {:?},\n offset: {:?},\n total_num_of_users: {:?},\n total_num_of_batches: {:?}",
        self.cfg, self.num_of_docs, self.num_of_batches_per_doc, self.file_idx, self.offset, self.total_num_of_users, self.total_num_of_batches);
    }
}

impl AccountParser for FilesParser {
    fn total_num_of_users(&self) -> usize {
        self.total_num_of_users
    }

    /// `offset` is to the global user vectors;
    fn read_n_accounts(&mut self, offset: usize, n: usize) -> Vec<Account> {
        debug!("read with offset: {:?}, account_num: {:?}", offset, n);
        self.log_state();
        // to make it simpler, we assume only read by a multiple of batch size;
        // if try to read cross multiple docs; simply can run this function multiple times;
        let n = if n > self.num_of_batches_per_doc * self.cfg.batch_size {
            self.num_of_batches_per_doc * self.cfg.batch_size
        } else {
            n
        };
        assert!(n % self.cfg.batch_size == 0);
        let min_offset: usize = self.file_idx * self.cfg.batch_size * self.num_of_batches_per_doc;
        assert!(offset >= min_offset);

        let acct_len =
            if offset + n > self.total_num_of_users { self.total_num_of_users - offset } else { n };

        let mut result = vec![Account::get_empty_account(self.cfg.num_of_tokens); acct_len];
        if (n + offset - min_offset) <= (self.buffered_accounts.len()) {
            // we have enough account in the buffer
            result.clone_from_slice(&self.buffered_accounts[self.offset..(self.offset + n)]);
            self.offset += n;
        } else {
            if self.offset < self.buffered_accounts.len() {
                let remain_len = self.buffered_accounts.len() - self.offset;
                result[0..remain_len].clone_from_slice(&self.buffered_accounts[(self.offset)..]);
            }
            let missing_len = result.len() - (self.buffered_accounts.len() - self.offset);
            debug!(
                "result len: {:?}, self.offset: {:?}, missing_len: {:?}",
                result.len(),
                self.offset,
                missing_len
            );
            if self.file_idx < (self.num_of_docs - 1) {
                // load the next file; TODO: assert_eq!(accounts_len, last_doc_account_num);
                self.file_idx += 1;
                self.buffered_accounts =
                    read_json_into_accounts_vec(self.docs[self.file_idx].to_str().unwrap());
                result.clone_from_slice(&self.buffered_accounts[0..missing_len]);
                self.offset = missing_len;
            } else {
                self.offset += result.len();
            }
        }
        result
    }
}

fn list_json_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut json_files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "json" {
                        json_files.push(path);
                    }
                }
            } else if path.is_dir() {
                json_files.extend(list_json_files(&path)?);
            }
        }
    }
    json_files.sort();
    Ok(json_files)
}

/// Read a json file and return the vec of associated accounts.
pub fn read_json_into_accounts_vec(path: &str) -> Vec<Account> {
    let parsed_data = read_json_file_into_map(path);
    parse_exchange_state(&parsed_data)
}

/// Reads a json file into a json string.
fn read_json_file_into_map(path: &str) -> Vec<BTreeMap<String, Value>> {
    let file = File::open(path);
    match file {
        Ok(f) => {
            let reader = BufReader::new(f);
            // Deserialize the binary data to a struct
            serde_json::from_reader(reader).expect("Unable to parse Json Data")
        }
        Err(e) => {
            error!("file open error, {:?}", e);
            panic!("File not found at specified path");
        }
    }
}

/// Parses the exchanges state at some snapshot and returns.
fn parse_exchange_state(parsed_data: &Vec<BTreeMap<String, Value>>) -> Vec<Account> {
    let mut accounts_data: Vec<Account> = Vec::new();
    for obj in parsed_data {
        let mut account_id = "";
        let mut inner_vec: Vec<F> = Vec::new();
        for (key, value) in obj.iter() {
            if key != "id" {
                if let Some(number_str) = value.as_str() {
                    match number_str.parse::<u64>() {
                        Ok(number) => inner_vec.push(F::from_canonical_u64(number)),
                        Err(e) => {
                            error!("Error in parsing token value number: {:?}", e);
                            panic!("Error in parsing token value number: {:?}", e);
                        }
                    }
                } else {
                    error!("Error in parsing string from json: {:?}", value);
                    panic!("Error in parsing string from json: {:?}", value);
                }
            } else {
                account_id = value.as_str().unwrap();
            }
        }
        // todo:: currently, we fill debt all to zero
        let asset_len = inner_vec.len();
        accounts_data.push(Account {
            id: account_id.into(),
            equity: inner_vec,
            debt: vec![F::ZERO; asset_len],
        });
    }
    accounts_data
}

#[cfg(test)]
mod test {

    use crate::parser::read_json_into_accounts_vec;

    use super::{parse_exchange_state, read_json_file_into_map};

    #[test]
    pub fn test_read_json_file_into_map() {
        let path = "../../test-data/batch0.json";
        let maps = read_json_file_into_map(path);

        let id_0 = "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ad";
        let parsed_id_0 = maps.get(0).unwrap().get("id").unwrap();
        assert_eq!(id_0, parsed_id_0);

        let id_1 = "47db1d296a7c146eab653591583a9a4873c626d8de47ae11393edd153e40f1ed";
        let parsed_id_1 = maps.get(1).unwrap().get("id").unwrap();
        assert_eq!(id_1, parsed_id_1);
    }

    #[test]
    pub fn test_parse_exchange_state() {
        let path = "../../test-data/batch0.json";
        let maps = read_json_file_into_map(path);
        let accounts = parse_exchange_state(&maps);

        let id_0 = "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ad";
        let account_0 = accounts.get(0).unwrap();
        assert_eq!(id_0, account_0.id);

        let id_1 = "47db1d296a7c146eab653591583a9a4873c626d8de47ae11393edd153e40f1ed";
        let account_1 = accounts.get(1).unwrap();
        assert_eq!(id_1, account_1.id);
    }

    #[test]
    pub fn test_read_json_into_accounts_vec() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(&path);

        let id_0 = "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ad";
        let account_0 = accounts.get(0).unwrap();
        assert_eq!(id_0, account_0.id);

        let id_1 = "47db1d296a7c146eab653591583a9a4873c626d8de47ae11393edd153e40f1ed";
        let account_1 = accounts.get(1).unwrap();
        assert_eq!(id_1, account_1.id);
    }
}

pub struct RandomAccountParser {
    pub total_num_of_users: usize,
    pub num_of_tokens: usize,
}
impl RandomAccountParser {
    pub fn new(total_num_of_users: usize, num_of_tokens: usize) -> Self {
        RandomAccountParser { total_num_of_users: total_num_of_users, num_of_tokens: num_of_tokens }
    }
}

impl AccountParser for RandomAccountParser {
    fn total_num_of_users(&self) -> usize {
        self.total_num_of_users
    }
    /// `offset` is to the global user vectors;
    fn read_n_accounts(&mut self, offset: usize, n: usize) -> Vec<Account> {
        let n = std::cmp::min(n, self.total_num_of_users - offset);
        gen_accounts_with_random_data(n, self.num_of_tokens)
    }
}
