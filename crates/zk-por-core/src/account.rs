use crate::{
    database::{PoRDB, UserId},
    types::F,
};
use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::Hasher,
};
use plonky2_field::types::{Field, PrimeField64};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A struct representing a users account. It represents their equity and debt as a Vector of goldilocks field elements.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String, // 256 bit hex string
    pub equity: Vec<F>,
    pub debt: Vec<F>,
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Account", 3)?;
        state.serialize_field("id", &self.id)?;
        // Custom serialization for equity and debt to ensure they are serialized in a specific format if needed
        let equity_as_strings: Vec<String> = self
            .equity
            .iter()
            .map(|e| {
                let num = e.to_canonical_u64();
                num.to_string()
            })
            .collect();
        state.serialize_field("equity", &equity_as_strings)?;

        let debt_as_strings: Vec<String> = self
            .debt
            .iter()
            .map(|e| {
                let num = e.to_canonical_u64();
                num.to_string()
            })
            .collect();
        state.serialize_field("debt", &debt_as_strings)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Account {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InnerAccount {
            id: String,
            equity: Vec<String>,
            debt: Vec<String>,
        }

        let helper = InnerAccount::deserialize(deserializer)?;
        let equity = helper
            .equity
            .iter()
            .map(|e| F::from_canonical_u64(u64::from_str_radix(e, 10).unwrap()))
            .collect();
        let debt = helper
            .debt
            .iter()
            .map(|e| F::from_canonical_u64(u64::from_str_radix(e, 10).unwrap()))
            .collect();

        Ok(Account { id: helper.id, equity: equity, debt: debt })
    }
}

impl Account {
    /// Gets the account hash for a given account.
    pub fn get_hash(&self) -> HashOut<F> {
        let sum_equity = self.equity.iter().fold(F::ZERO, |acc, x| acc + *x);

        let sum_debt = self.debt.iter().fold(F::ZERO, |acc, x| acc + *x);

        let id = self.get_user_id_in_field();

        #[allow(clippy::useless_vec)]
        let hash =
            PoseidonHash::hash_no_pad(vec![id, vec![sum_equity, sum_debt]].concat().as_slice());

        hash
    }

    pub fn get_empty_account_with_user_id(user_id: String, num_of_tokens: usize) -> Account {
        Self {
            id: user_id,
            equity: vec![F::default(); num_of_tokens],
            debt: vec![F::default(); num_of_tokens],
        }
    }

    pub fn get_empty_account(num_of_tokens: usize) -> Account {
        Self {
            id: "0".repeat(64),
            equity: vec![F::default(); num_of_tokens],
            debt: vec![F::default(); num_of_tokens],
        }
    }

    /// Gets a user id as a vec of 5 GF elements.
    pub fn get_user_id_in_field(&self) -> Vec<F> {
        assert!(self.id.len() == 64);
        #[allow(clippy::useless_vec)]
        let segments = vec![
            self.id[0..14].to_string(),  // First 56 bits (14 hex chars)
            self.id[14..28].to_string(), // Second 56 bits
            self.id[28..42].to_string(), // Third 56 bits
            self.id[42..56].to_string(), // Fourth 56 bits
            self.id[56..64].to_string(), // Remaining 32 bits (8 hex chars, fits in 56 bits)
        ];

        segments
            .iter()
            .map(|seg| F::from_canonical_u64(u64::from_str_radix(seg, 16).unwrap()))
            .collect::<Vec<F>>()
    }
}

pub fn persist_account_id_to_gmst_pos(
    db: &mut Box<dyn PoRDB>,
    accounts: &Vec<Account>,
    start_idx: usize,
) {
    let user_batch = accounts
        .iter()
        .enumerate()
        .map(|(i, acct)| {
            let user_id = UserId::from_hex_string(acct.id.to_string()).unwrap();
            // tracing::debug!("persist account {:?} with index: {:?}", acct.id, i + start_idx);
            (user_id, (i + start_idx) as u32)
        })
        .collect::<Vec<(UserId, u32)>>();
    db.add_batch_users(user_batch);
}

/// Generates num_accounts number of accounts with num_assets of assets (with equity and debt being seperate vecs)
pub fn gen_accounts_with_random_data(num_accounts: usize, num_assets: usize) -> Vec<Account> {
    let mut accounts: Vec<Account> = Vec::new();
    let mut rng = rand::thread_rng(); // Create a random number generator
    for _ in 0..num_accounts {
        let mut equities = Vec::new();
        let mut debts = Vec::new();
        for _ in 0..num_assets {
            let equity = rng.gen_range(1..1000);
            let debt = equity - 1; // such that debt is always less than equity
            equities.push(F::from_canonical_u32(equity));
            debts.push(F::from_canonical_u32(debt));
        }

        let mut bytes = [0u8; 32]; // 32 bytes * 2 hex chars per byte = 64 hex chars
        rng.fill(&mut bytes);
        #[allow(clippy::format_collect)]
        let account_id = bytes.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();
        accounts.push(Account { id: account_id, equity: equities, debt: debts });
    }
    accounts
}

pub fn gen_empty_accounts(batch_size: usize, num_assets: usize) -> Vec<Account> {
    let accounts =
        vec![
            Account::get_empty_account_with_user_id(UserId::rand().to_string(), num_assets);
            batch_size
        ];
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_account_json_marshalling() {
        // Step 1: Create an instance of `Account`
        let original_account = Account {
            id: "1".to_owned(), // Assuming `id` is of type that implements `Serialize` and `Deserialize`
            equity: vec![F::from_canonical_u64(0), F::from_canonical_u64(1)],
            debt: vec![F::from_canonical_u64(0), F::from_canonical_u64(2)],
        };

        // Step 2: Serialize the `Account` instance to a JSON string
        let json_string = serde_json::to_string(&original_account).unwrap();

        // Step 3: Deserialize the JSON string back into an `Account` instance
        let deserialized_account: Account = serde_json::from_str(&json_string).unwrap();

        // Step 4: Assert that the original and deserialized instances are equal
        assert_eq!(original_account.id, deserialized_account.id);
        assert_eq!(original_account.equity, deserialized_account.equity);
        assert_eq!(original_account.debt, deserialized_account.debt);
    }
}
