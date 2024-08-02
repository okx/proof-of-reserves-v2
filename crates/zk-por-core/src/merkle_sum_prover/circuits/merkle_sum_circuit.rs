use plonky2::{
    hash::{
        hash_types::{HashOutTarget, NUM_HASH_OUT_ELTS},
        poseidon::PoseidonHash,
    },
    iop::target::Target,
    plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData},
};

use crate::types::{C, D, F};

use super::{
    account_circuit::{AccountSumTargets, AccountTargets},
    circuit_utils::assert_non_negative_unsigned,
};
use crate::circuit_config::STANDARD_CONFIG;
use plonky2_field::types::Field;

/// A node in the merkle sum tree, contains the total amount of equity (in usd) and the total amount of debt (in usd) and the hash.
///
/// The hash is Hash(hash_left, hash_right).
///
/// The amount of equity and amount of debt is the sum of the equity and debt of the children.
#[derive(Debug, Copy, Clone)]
pub struct MerkleSumNodeTarget {
    pub sum_equity: Target,
    pub sum_debt: Target,
    pub hash: HashOutTarget,
}

impl MerkleSumNodeTarget {
    pub fn default() -> Self {
        MerkleSumNodeTarget {
            sum_equity: Target::default(),
            sum_debt: Target::default(),
            hash: [Target::default(); NUM_HASH_OUT_ELTS].into(),
        }
    }

    /// Given children nodes, generate the MerkleSumNodeTarget
    pub fn get_parent_from_children<const N: usize>(
        builder: &mut CircuitBuilder<F, D>,
        children: [&MerkleSumNodeTarget; N],
    ) -> MerkleSumNodeTarget {
        let mut sum_equity = builder.constant(F::ZERO);
        let mut sum_debt = builder.constant(F::ZERO);
        let mut hash_inputs = Vec::new();
        children.into_iter().for_each(|child| {
            sum_equity = builder.add(sum_equity, child.sum_equity);
            sum_debt = builder.add(sum_debt, child.sum_debt);

            // Ensure the amount of equity at this node is greater than the total amount of debt
            let diff_between_equity_debt = builder.sub(sum_equity, sum_debt);
            assert_non_negative_unsigned(builder, diff_between_equity_debt);

            // Ensure no overflow. We only need to check one child since in any overflow, the new value will be less than both the left and
            // right children.
            let diff_between_equity_child_and_sum = builder.sub(sum_equity, child.sum_equity);
            assert_non_negative_unsigned(builder, diff_between_equity_child_and_sum);
            let diff_between_debt_child_and_sum = builder.sub(sum_debt, child.sum_debt);
            assert_non_negative_unsigned(builder, diff_between_debt_child_and_sum);

            hash_inputs.extend(child.hash.elements.iter());
        });
        let hash = builder.hash_n_to_hash_no_pad::<PoseidonHash>(hash_inputs);
        MerkleSumNodeTarget { sum_equity, sum_debt, hash }
    }

    /// Get a merkle sum node target from account sum targets.
    pub fn get_node_from_account_targets(
        builder: &mut CircuitBuilder<F, D>,
        account_targets: &AccountSumTargets,
    ) -> MerkleSumNodeTarget {
        let hash_inputs = vec![account_targets.sum_equity, account_targets.sum_debt];

        let hash = builder.hash_n_to_hash_no_pad::<PoseidonHash>(hash_inputs);
        MerkleSumNodeTarget {
            sum_equity: account_targets.sum_equity,
            sum_debt: account_targets.sum_debt,
            hash,
        }
    }
    pub fn registered_as_public_inputs(&self, builder: &mut CircuitBuilder<F, D>) {
        builder.register_public_input(self.sum_equity);
        builder.register_public_input(self.sum_debt);
        builder.register_public_inputs(self.hash.elements.as_slice());
    }
}

impl From<MerkleSumNodeTarget> for Vec<Target> {
    fn from(node: MerkleSumNodeTarget) -> Vec<Target> {
        vec![vec![node.sum_equity, node.sum_debt], node.hash.elements.to_vec()].concat()
    }
}

impl From<Vec<Target>> for MerkleSumNodeTarget {
    /// the parsing order must be consistent with the order of public input registration in `registered_as_public_inputs`
    fn from(inputs: Vec<Target>) -> MerkleSumNodeTarget {
        let mut iter = inputs.into_iter();
        let sum_equity_target = iter.next().unwrap();
        let sum_debt_target = iter.next().unwrap();
        let hash_target = HashOutTarget::from_vec(iter.by_ref().take(NUM_HASH_OUT_ELTS).collect());

        MerkleSumNodeTarget {
            sum_equity: sum_equity_target,
            sum_debt: sum_debt_target,
            hash: hash_target,
        }
    }
}
/// We can represent the Merkle Sum Tree as a vector of merkle sum nodes, with the root being the last node in the vector.    
pub struct MerkleSumTreeTarget {
    pub sum_tree: Vec<MerkleSumNodeTarget>,
}

impl MerkleSumTreeTarget {
    pub fn get_root(&self) -> &MerkleSumNodeTarget {
        self.sum_tree.last().unwrap()
    }

    /// Register the root hash, sum_equity and sum_debt as public inputs to be used in recursive proving.
    pub fn register_public_inputs(&self, builder: &mut CircuitBuilder<F, D>) {
        let root = self.get_root();
        builder.register_public_input(root.sum_equity);
        builder.register_public_input(root.sum_debt);
        builder.register_public_inputs(&root.hash.elements);
    }
}

/// Builds a merkle sum tree of a given size (based on the number of leaves). It will build the merkle sum tree on top of the leaves vector
/// in order to do the task in place. There is no return value as the input leaves vector is mutated.
pub fn build_merkle_sum_tree(
    builder: &mut CircuitBuilder<F, D>,
    leaves: &mut Vec<MerkleSumNodeTarget>,
) {
    let num_leaves = leaves.len();

    for i in num_leaves..(num_leaves * 2 - 1) {
        let left_child_index = 2 * (i - num_leaves);
        let right_child_index = 2 * (i - num_leaves) + 1;
        let left_child = leaves.get(left_child_index).unwrap();
        let right_child = leaves.get(right_child_index).unwrap();
        leaves.push(MerkleSumNodeTarget::get_parent_from_children(
            builder,
            [left_child, right_child],
        ));
    }
}

/// Given a list of account targets, build the corresponding merkle sum tree.
pub fn build_merkle_sum_tree_from_account_targets(
    builder: &mut CircuitBuilder<F, D>,
    accounts: &mut Vec<AccountSumTargets>,
) -> MerkleSumTreeTarget {
    let mut leaves: Vec<MerkleSumNodeTarget> = accounts
        .iter()
        .map(|x| MerkleSumNodeTarget::get_node_from_account_targets(builder, x))
        .collect();

    build_merkle_sum_tree(builder, &mut leaves);

    let tree = MerkleSumTreeTarget { sum_tree: leaves };

    tree.register_public_inputs(builder);

    return tree;
}

pub fn build_merkle_sum_tree_circuit(
    num_of_leaves: usize,
    asset_num: usize,
) -> (CircuitData<F, C, D>, Vec<AccountTargets>) {
    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let mut account_targets: Vec<AccountTargets> = Vec::new();
    (0..num_of_leaves).for_each(|_| {
        let equity_targets = builder.add_virtual_targets(asset_num);
        let debt_targets = builder.add_virtual_targets(asset_num);
        let account_target = AccountTargets { equity: equity_targets, debt: debt_targets };
        account_targets.push(account_target);
    });
    let mut account_sum_targets: Vec<AccountSumTargets> = account_targets
        .iter()
        .map(|x| AccountSumTargets::from_account_target(x, &mut builder))
        .collect();

    _ = build_merkle_sum_tree_from_account_targets(&mut builder, &mut account_sum_targets);
    let circuit_data = builder.build::<C>();
    (circuit_data, account_targets)
}

#[cfg(test)]
pub mod test {
    use crate::{
        merkle_sum_prover::circuits::{
            account_circuit::{AccountSumTargets, AccountTargets},
            circuit_utils::run_circuit_test,
        },
        parser::read_json_into_accounts_vec,
    };

    use super::MerkleSumNodeTarget;

    #[test]
    pub fn test_merkle_sum_node() {
        run_circuit_test(|builder, pw| {
            let path = "../../test-data/batch0.json";
            let accounts = read_json_into_accounts_vec(path);

            let account_target_1 =
                AccountTargets::new_from_account(accounts.get(0).unwrap(), builder);
            let account_target_2 =
                AccountTargets::new_from_account(accounts.get(1).unwrap(), builder);

            let account_sum_target_1 =
                AccountSumTargets::from_account_target(&account_target_1, builder);
            let account_sum_target_2 =
                AccountSumTargets::from_account_target(&account_target_2, builder);

            let merkle_sum_node_target_1 =
                MerkleSumNodeTarget::get_node_from_account_targets(builder, &account_sum_target_1);
            let merkle_sum_node_target_2 =
                MerkleSumNodeTarget::get_node_from_account_targets(builder, &account_sum_target_2);

            let merkle_sum_node_target_3 = MerkleSumNodeTarget::get_parent_from_children(
                builder,
                [&merkle_sum_node_target_1, &merkle_sum_node_target_2],
            );

            let sum_equity =
                builder.add(account_sum_target_1.sum_equity, account_sum_target_2.sum_equity);
            let sum_debt =
                builder.add(account_sum_target_1.sum_debt, account_sum_target_2.sum_debt);

            builder.connect(merkle_sum_node_target_3.sum_equity, sum_equity);
            builder.connect(merkle_sum_node_target_3.sum_debt, sum_debt);

            account_target_1.set_account_targets(accounts.get(0).unwrap(), pw);
            account_target_2.set_account_targets(accounts.get(1).unwrap(), pw);
        });
    }
}
