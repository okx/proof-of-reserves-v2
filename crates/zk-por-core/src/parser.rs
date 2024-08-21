use super::account::{gen_accounts_with_random_data, Account};
use crate::types::F;
use plonky2_field::types::Field;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    fs::File,
    io::BufReader,
    ops::Div,
    path::{Path, PathBuf},
};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct FilesCfg {
    pub dir: PathBuf,
    pub batch_size: usize,
    pub tokens: Vec<String>,
}

pub trait AccountParser {
    fn read_n_accounts(
        &mut self,
        offset: usize,
        n: usize,
        fm: &impl JsonFileManager,
    ) -> Vec<Account>;
    fn total_num_of_users(&self) -> usize;
}

#[derive(Debug)]
pub struct FileManager {}

pub trait JsonFileManager {
    fn list_json_files(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>>;
    fn read_json_into_accounts_vec(&self, path: &str, tokens : &Vec<String>) -> Vec<Account>;
    fn read_json_file_into_map(&self, path: &str) -> Vec<BTreeMap<String, Value>>;
}

impl JsonFileManager for FileManager {
    fn list_json_files(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>> {
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
                    json_files.extend(self.list_json_files(&path)?);
                }
            }
        }
        json_files.sort();
        Ok(json_files)
    }

    /// Read a json file and return the vec of associated accounts.
    /// tokens is a list of all possible token names. It is used to fill the account with zero for missing tokens.
    fn read_json_into_accounts_vec(&self, path: &str, tokens : &Vec<String>) -> Vec<Account> {
        let parsed_data = self.read_json_file_into_map(path);
        parse_exchange_state(&parsed_data, tokens)
    }

    /// Reads a json file into a json string.
    fn read_json_file_into_map(&self, path: &str) -> Vec<BTreeMap<String, Value>> {
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
}

#[derive(Debug)]
pub struct FileAccountReader {
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

impl FileAccountReader {
    pub fn new(cfg: FilesCfg, fm: &impl JsonFileManager) -> Self {
        let user_data_path = std::path::Path::new(&(cfg.dir));
        if !user_data_path.exists() {
            panic!("dir: {:?} does not exist", user_data_path);
        }
        let json_files = fm.list_json_files(user_data_path);
        let mut parser = Self {
            buffered_accounts: vec![],
            last_doc_accounts: vec![],
            file_idx: 0,
            offset: 0,
            num_of_docs: 0,
            cfg: cfg.clone(),
            num_of_batches_per_doc: 0,
            total_num_of_users: 0,
            total_num_of_batches: 0,
            docs: vec![],
        };

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
                    fm.read_json_into_accounts_vec(parser.docs[0].to_str().unwrap(), &cfg.tokens);
                let first_doc_accounts_len = first_doc_accounts.len();

                if doc_len > 1 {
                    assert_eq!(first_doc_accounts_len % parser.cfg.batch_size, 0);
                }
                parser.num_of_batches_per_doc = first_doc_accounts_len.div(parser.cfg.batch_size);
                parser.buffered_accounts = first_doc_accounts;
                parser.file_idx = 0;

                if doc_len > 1 {
                    let last_doc_accounts =
                        fm.read_json_into_accounts_vec(parser.docs[doc_len - 1].to_str().unwrap(), &cfg.tokens);
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

impl AccountParser for FileAccountReader {
    fn total_num_of_users(&self) -> usize {
        self.total_num_of_users
    }

    /// `offset` is to the global user vectors;
    fn read_n_accounts(
        &mut self,
        offset: usize,
        n: usize,
        fm: &impl JsonFileManager,
    ) -> Vec<Account> {
        debug!("read with offset: {:?}, account_num: {:?}", offset, n);
        self.log_state();
        // to make it simpler, we assume only read by a multiple of batch size;
        // if try to read cross multiple docs; simply can run this function multiple times;
        assert!(n % self.cfg.batch_size == 0);
        let min_offset: usize = self.file_idx * self.cfg.batch_size * self.num_of_batches_per_doc;
        assert!(offset >= min_offset);

        let acct_len =
            if offset + n > self.total_num_of_users { self.total_num_of_users - offset } else { n };
        let num_of_tokens = self.cfg.tokens.len();
        let mut result = vec![Account::get_empty_account(num_of_tokens); acct_len];
        if (n + offset - min_offset) <= (self.buffered_accounts.len()) {
            // we have enough account in the buffer
            result.clone_from_slice(&self.buffered_accounts[self.offset..(self.offset + n)]);
            self.offset += n;
        } else {
            let mut filled_len = 0;

            if self.offset < self.buffered_accounts.len() {
                let filled_len = self.buffered_accounts.len() - self.offset;
                result[0..filled_len].clone_from_slice(&self.buffered_accounts[(self.offset)..]);
            }
            let mut missing_len = result.len() - (self.buffered_accounts.len() - self.offset);
            debug!(
                "result len: {:?}, self.offset: {:?}, missing_len: {:?}",
                result.len(),
                self.offset,
                missing_len
            );

            while missing_len > 0 {
                let to_read =
                    std::cmp::min(self.num_of_batches_per_doc * self.cfg.batch_size, missing_len);
                if self.file_idx < (self.num_of_docs - 1) {
                    // load the next file; TODO: assert_eq!(accounts_len, last_doc_account_num);
                    self.file_idx += 1;
                    self.buffered_accounts =
                        fm.read_json_into_accounts_vec(self.docs[self.file_idx].to_str().unwrap(), &self.cfg.tokens);
                    result[filled_len..(filled_len + to_read)]
                        .clone_from_slice(&self.buffered_accounts[0..to_read]);
                    filled_len += to_read;
                    self.offset = to_read;
                } else {
                    self.offset += result.len();
                    break;
                }
                missing_len -= to_read;
            }
        }
        result
    }
}

/// Parses the exchanges state at some snapshot and returns.
fn parse_exchange_state(parsed_data: &Vec<BTreeMap<String, Value>>, tokens : &Vec<String>) -> Vec<Account> {
    let mut accounts_data: Vec<Account> = Vec::new();
    for obj in parsed_data {
        accounts_data.push(parse_account_state(obj, tokens));
    }
    accounts_data
}

/// Parses the exchanges state at some snapshot and returns.
pub fn parse_account_state(parsed_data: &BTreeMap<String, Value>, tokens : &Vec<String>) -> Account {
    let account_id = parsed_data.get("id").expect(format!("Account {:?} dont have key `id`", parsed_data).as_str()).as_str().unwrap();
    let token_num = tokens.len();

    let equities = parsed_data.get("equity").expect(format!("Account {:?} dont have key `equity`", parsed_data).as_str()).as_object().unwrap();
    let mut parsed_equities = Vec::new();
    for token in tokens.iter() {
        if let Some(val) = equities.get(token) {
            let parsed_equity = F::from_canonical_u64(val.as_str().unwrap().parse::<u64>().unwrap());
            parsed_equities.push(parsed_equity);
        } else {
            panic!("fail to find equity for token: {:?} in accountID {:?}", token, account_id);
        }
    }

    // if there is no debt, we fill it with zero
    let mut parsed_debts = vec![F::ZERO; token_num];
    if let Some(debts) = parsed_data.get("debt") {
        let debts = debts.as_object().unwrap();
        for token in tokens.iter() {
            if let Some(val) = debts.get(token) {
                let parsed_debt = F::from_canonical_u64(val.as_str().unwrap().parse::<u64>().unwrap());
                parsed_debts.push(parsed_debt);
            } else {
                panic!("fail to find debt for token: {:?} in accountID {:?}", token, account_id);
            }
        }
    }

    Account { id: account_id.into(), equity: parsed_equities, debt: parsed_debts }
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
    fn read_n_accounts(
        &mut self,
        offset: usize,
        n: usize,
        _: &impl JsonFileManager,
    ) -> Vec<Account> {
        let n = std::cmp::min(n, self.total_num_of_users - offset);
        gen_accounts_with_random_data(n, self.num_of_tokens)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        account::Account,
        parser::{parse_exchange_state, FileManager, FilesCfg},
    };
    use mockall::*;
    use serde_json::Value;
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        str::FromStr,
    };

    use super::{AccountParser, FileAccountReader, JsonFileManager};

    #[test]
    pub fn test_read_json_file_into_map() {
        let fm = FileManager {};
        let path = "../../test-data/batch0.json";
        let maps = fm.read_json_file_into_map(path);

        let id_0 = "4282aed0318e3271db2649f3a4a6855d9f83285d04ea541d741fd53a602eb73e";
        let parsed_id_0 = maps.get(0).unwrap().get("id").unwrap();
        assert_eq!(id_0, parsed_id_0);

        let id_1 = "bfad15056e9c14831ee4351f180b7cbd141a1b372ba8696c8505f7335282126d";
        let parsed_id_1 = maps.get(1).unwrap().get("id").unwrap();
        assert_eq!(id_1, parsed_id_1);
    }

    #[test]
    pub fn test_parse_exchange_state() {
        let fm = FileManager {};
        let path = "../../test-data/batch0.json";
        let maps = fm.read_json_file_into_map(path);
        let tokens = vec!["BTC".to_string(), "ETH".to_string()];
        let accounts = parse_exchange_state(&maps, &tokens);

        let id_0 = "4282aed0318e3271db2649f3a4a6855d9f83285d04ea541d741fd53a602eb73e";
        let account_0 = accounts.get(0).unwrap();
        assert_eq!(id_0, account_0.id);

        let id_1 = "bfad15056e9c14831ee4351f180b7cbd141a1b372ba8696c8505f7335282126d";
        let account_1 = accounts.get(1).unwrap();
        assert_eq!(id_1, account_1.id);
    }

    #[test]
    pub fn test_read_json_into_accounts_vec() {
        let fm = FileManager {};
        let path = "../../test-data/batch0.json";
        let tokens = vec!["BTC".to_string(), "ETH".to_string()];
        let accounts = fm.read_json_into_accounts_vec(&path, &tokens);

        let id_0 = "4282aed0318e3271db2649f3a4a6855d9f83285d04ea541d741fd53a602eb73e";
        let account_0 = accounts.get(0).unwrap();
        assert_eq!(id_0, account_0.id);

        let id_1 = "bfad15056e9c14831ee4351f180b7cbd141a1b372ba8696c8505f7335282126d";
        let account_1 = accounts.get(1).unwrap();
        assert_eq!(id_1, account_1.id);
    }

    mock! {
      pub FileManager {}

      impl JsonFileManager for FileManager {
          fn list_json_files(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>>;
          fn read_json_into_accounts_vec(&self, path: &str, tokens : &Vec<String>) -> Vec<Account>;
          fn read_json_file_into_map(&self, path: &str) -> Vec<BTreeMap<String, Value>>;
      }
    }

    #[test]
    fn test_file_account_reader() {
        let mut mock_file_manager = MockFileManager::new();
        mock_file_manager.expect_list_json_files().returning(|_| {
            let paths = vec![
                std::path::PathBuf::from_str("file0").unwrap(),
                std::path::PathBuf::from_str("file1").unwrap(),
                std::path::PathBuf::from_str("file2").unwrap(),
                std::path::PathBuf::from_str("file3").unwrap(),
                std::path::PathBuf::from_str("file4").unwrap(),
                std::path::PathBuf::from_str("file5").unwrap(),
            ];
            Ok(paths)
        });

        mock_file_manager.expect_read_json_into_accounts_vec().times(1).returning(|_, _| {
            let accounts = vec![Account::get_empty_account(20); 4];
            accounts
        });
        mock_file_manager.expect_read_json_into_accounts_vec().times(1).returning(|_, _| {
            let accounts = vec![Account::get_empty_account(20); 3];
            accounts
        });

        let dir = tempdir::TempDir::new("user_input_test").unwrap().into_path();

        let mut file_acct_reader = FileAccountReader::new(
            FilesCfg { dir, batch_size: 4, tokens : vec!["BTC".to_owned(), "ETH".to_owned()] },
            &mock_file_manager,
        );
        assert_eq!(file_acct_reader.total_num_of_users(), 23);

        mock_file_manager.expect_read_json_into_accounts_vec().times(1).returning(|_, _| {
            let accounts = vec![Account::get_empty_account(20); 4];
            accounts
        });
        let users = file_acct_reader.read_n_accounts(0, 8, &mock_file_manager);
        assert_eq!(users.len(), 8);

        mock_file_manager.expect_read_json_into_accounts_vec().times(1).returning(|_, _| {
            let accounts = vec![Account::get_empty_account(20); 4];
            accounts
        });
        let users = file_acct_reader.read_n_accounts(8, 4, &mock_file_manager);
        assert_eq!(users.len(), 4);

        mock_file_manager.expect_read_json_into_accounts_vec().times(3).returning(|_, _| {
            let accounts = vec![Account::get_empty_account(20); 4];
            accounts
        });
        let users = file_acct_reader.read_n_accounts(12, 12, &mock_file_manager);
        assert_eq!(users.len(), 11);
    }
}
