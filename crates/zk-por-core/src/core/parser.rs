use std::{collections::BTreeMap, fs::File, io::BufReader};

use plonky2_field::types::Field;
use serde_json::Value;

use crate::types::F;

use super::account::Account;

/// Read a json file and return the vec of associated accounts.
pub fn read_json_into_account(path: &str)-> Vec<Account>{
    let parsed_data = read_json_file_into_json_str(path);
    let accounts_data = parse_exchange_state(&parsed_data);
    accounts_data
}

/// Reads a json file into a json string.
fn read_json_file_into_json_str(path: &str)-> Vec<BTreeMap<String, Value>>{
    let file = File::open(path).expect("Cannot read file");
    let reader = BufReader::new(file);

    // Deserialize the binary data to a struct
    serde_json::from_reader(reader).expect("Unable to parse Json Data")
}

/// Parses the exchanges state at some snapshot and returns. 
fn parse_exchange_state(parsed_data: &Vec<BTreeMap<String, Value>>)-> Vec<Account>{
    let mut accounts_data: Vec<Account> = Vec::new();
    for obj in parsed_data {
        let mut account_id = "";
        let mut inner_vec: Vec<F> = Vec::new();
        for (key, value) in obj.iter() {
            if key != "id" {
                if let Some(number_str) = value.as_str() {
                    if let Ok(number) = number_str.parse::<u64>() {
                        inner_vec.push(F::from_canonical_u64(number));
                    }
                }
            }else{
                account_id = value.as_str().unwrap();
            }
        }
        accounts_data.push(Account{
            id: account_id.into(),
            assets: inner_vec,
            debt: Vec::new()
        });

    }
    accounts_data
}
