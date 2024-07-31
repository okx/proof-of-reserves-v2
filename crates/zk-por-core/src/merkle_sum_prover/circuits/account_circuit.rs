use plonky2::{
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::circuit_builder::CircuitBuilder,
};

use crate::{
    account::Account,
    types::{D, F},
};

use super::circuit_utils::assert_non_negative_unsigned;

#[derive(Debug, Clone)]
/// Targets representing a users account, where their equity and debt are split into individual tokens.
pub struct AccountTargets {
    pub equity: Vec<Target>,
    pub debt: Vec<Target>,
}

impl AccountTargets {
    pub fn new_from_account(
        account: &Account,
        builder: &mut CircuitBuilder<F, D>,
    ) -> AccountTargets {
        let equity = builder.add_virtual_targets(account.equity.len());
        let debt = builder.add_virtual_targets(account.debt.len());

        AccountTargets { equity, debt }
    }

    pub fn set_account_targets(&self, account_info: &Account, pw: &mut PartialWitness<F>) {
        assert_eq!(self.equity.len(), account_info.equity.len());
        assert_eq!(self.debt.len(), account_info.debt.len());

        pw.set_target_arr(self.equity.as_slice(), account_info.equity.as_slice());
        pw.set_target_arr(self.debt.as_slice(), account_info.debt.as_slice());
    }
}

#[derive(Debug, Clone)]
/// Targets representing a users account, where their equity and liabilities are summed into 2 summed values.
pub struct AccountSumTargets {
    pub sum_equity: Target,
    pub sum_debt: Target,
}

impl AccountSumTargets {
    /// Given Account Targets, sum the account equity and liabilities and return a AccountSumTargets.
    pub fn from_account_target(
        account: &AccountTargets,
        builder: &mut CircuitBuilder<F, D>,
    ) -> AccountSumTargets {
        let sum_equity = account.equity.iter().fold(builder.zero(), |x, y| builder.add(x, *y));

        let sum_debt = account.debt.iter().fold(builder.zero(), |x, y| builder.add(x, *y));

        let diff_between_equity_debt = builder.sub(sum_equity, sum_debt);

        // Ensure the equity is greater than the debt. This works as long as we constrict our equity to 62 bits.
        assert_non_negative_unsigned(builder, diff_between_equity_debt);

        AccountSumTargets { sum_equity, sum_debt }
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        merkle_sum_prover::circuits::circuit_utils::run_circuit_test,
        parser::read_json_into_accounts_vec,
    };

    use super::{AccountSumTargets, AccountTargets};

    #[test]
    fn test_account_target() {
        run_circuit_test(|builder, pw| {
            let path = "../../test-data/batch0.json";
            let accounts = read_json_into_accounts_vec(path);

            let account_target =
                AccountTargets::new_from_account(accounts.get(0).unwrap(), builder);
            account_target.set_account_targets(accounts.get(0).unwrap(), pw);
        });
    }

    #[test]
    fn test_account_sum_target() {
        run_circuit_test(|builder, pw| {
            let path = "../../test-data/batch0.json";
            let accounts = read_json_into_accounts_vec(path);

            let account_target =
                AccountTargets::new_from_account(accounts.get(0).unwrap(), builder);

            let account_sum_target =
                AccountSumTargets::from_account_target(&account_target, builder);

            let total_equity =
                account_target.equity.iter().fold(builder.zero(), |x, y| builder.add(x, *y));
            let total_debt =
                account_target.debt.iter().fold(builder.zero(), |x, y| builder.add(x, *y));

            builder.connect(account_sum_target.sum_equity, total_equity);
            builder.connect(account_sum_target.sum_debt, total_debt);

            account_target.set_account_targets(accounts.get(0).unwrap(), pw);
        });
    }
}
