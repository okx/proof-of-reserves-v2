use crate::types::F;
use plonky2_field::types::Field;
use rand::Rng;

/// A struct representing a users account. It represents their equity and debt as a Vector of goldilocks field elements.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub equity: Vec<F>,
    pub debt: Vec<F>,
}

pub fn gen_accounts_with_random_data(
    num_accounts: usize,
    num_assets: usize,
) -> (Vec<Account>, u32, u32) {
    let mut accounts: Vec<Account> = Vec::new();
    let mut rng = rand::thread_rng(); // Create a random number generator
    let mut equity_sum = 0;
    let mut debt_sum = 0;
    for _ in 0..num_accounts {
        let mut equities = Vec::new();
        let mut debts = Vec::new();
        for _ in 0..num_assets {
            let equity = rng.gen_range(1..10);
            let debt = equity - 1; // such that debt is always less than equity
            equity_sum += equity;
            debt_sum += debt;
            equities.push(F::from_canonical_u32(equity));
            debts.push(F::from_canonical_u32(debt));
        }
        accounts.push(Account { id: "0".to_string(), equity: equities, debt: debts });
    }
    (accounts, equity_sum, debt_sum)
}
