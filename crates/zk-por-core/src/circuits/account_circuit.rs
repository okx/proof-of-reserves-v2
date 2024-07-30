use plonky2::{
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::circuit_builder::CircuitBuilder,
};

use crate::{
    core::account::Account,
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
    pub fn set_account_targets(&self, account_info: &Account, pw: &mut PartialWitness<F>) {
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
        let sum_equity = account
            .equity
            .iter()
            .fold(builder.zero(), |x, y| builder.add(x, *y));

        let sum_debt = account
            .debt
            .iter()
            .fold(builder.zero(), |x, y| builder.add(x, *y));

        let diff_between_equity_debt = builder.sub(sum_equity, sum_debt);

        // Ensure the equity is greater than the debt. This works as long as we constrict our equity to 62 bits.
        assert_non_negative_unsigned(builder, diff_between_equity_debt);

        AccountSumTargets {
            sum_equity,
            sum_debt,
        }
    }
}
