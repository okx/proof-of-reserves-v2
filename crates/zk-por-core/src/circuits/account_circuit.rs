use plonky2::{iop::{target::Target, witness::{PartialWitness, WitnessWrite}}, plonk::circuit_builder::CircuitBuilder};

use crate::{core::account::Account, types::{D, F}};

use super::circuit_utils::assert_non_negative_unsigned;

#[derive(Debug, Clone)]
/// Targets representing a users account, where their assets and liabilities are split into individual tokens.
pub struct AccountTargets{
    pub assets: Vec<Target>,
    pub debt: Vec<Target>,
}

impl AccountTargets{
    pub fn set_account_targets(
        &self,
        account_info: &Account,
        pw: &mut PartialWitness<F>
    ){
        pw.set_target_arr(self.assets.as_slice(), account_info.assets.as_slice());
        pw.set_target_arr(self.debt.as_slice(), account_info.debt.as_slice());
    }
}

#[derive(Debug, Clone)]
/// Targets representing a users account, where their assets and liabilities are summed into 2 summed values.
pub struct AccountSumTargets{
    pub sum_assets: Target,
    pub sum_debt: Target,
}

impl AccountSumTargets{
    /// Given Account Targets, sum the account assets and liabilities and return a AccountSumTargets.
    pub fn from_account_target(account: &AccountTargets, builder: &mut CircuitBuilder<F, D>)->AccountSumTargets{
        let sum_assets = account.assets.iter().fold(builder.zero(), |x, y| {
            builder.add(x, *y)
        });

        let sum_debt = account.assets.iter().fold(builder.zero(), |x, y| {
            builder.add(x, *y)
        });

        let diff_between_asset_debt = builder.sub(sum_assets, sum_debt);

        // Ensure the assets is greater than the debt. This works as long as we constrict our assets to 62 bits.
        assert_non_negative_unsigned(builder, diff_between_asset_debt);

        AccountSumTargets{
            sum_assets,
            sum_debt
        }
    }
}