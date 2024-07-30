use std::{collections::BTreeMap, fs::File, io::BufReader};

use plonky2_field::types::Field;
use serde_json::Value;

use crate::types::F;

use super::account::Account;

/// Read a json file and return the vec of associated accounts.
pub fn read_json_into_accounts_vec(path: &str)-> Vec<Account>{
    let parsed_data = read_json_file_into_map(path);
    let accounts_data = parse_exchange_state(&parsed_data);
    accounts_data
}

/// Reads a json file into a json string.
fn read_json_file_into_map(path: &str)-> Vec<BTreeMap<String, Value>>{
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
                    }else{
                        
                    }
                }
            }else{
                account_id = value.as_str().unwrap();
            }
        }
        accounts_data.push(Account{
            id: account_id.into(),
            equity: inner_vec,
            debt: Vec::new()
        });

    }
    accounts_data
}


#[cfg(test)]
mod test {
    use crate::core::parser::read_json_into_accounts_vec;

    use super::{parse_exchange_state, read_json_file_into_map};

    #[test]
    pub fn test_read_json_file_into_map(){
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
    pub fn test_parse_exchange_state(){
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
    pub fn test_read_json_into_accounts_vec(){
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

