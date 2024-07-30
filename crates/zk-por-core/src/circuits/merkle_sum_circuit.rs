use plonky2::{
    hash::{
        hash_types::{HashOutTarget, NUM_HASH_OUT_ELTS},
        poseidon::PoseidonHash,
    },
    iop::target::Target,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::types::{F,D};

use super::{account_circuit::AccountSumTargets, circuit_utils::assert_non_negative_unsigned};

/// A node in the merkle sum tree, contains the total amount of equity (in usd) and the total amount of debt (in usd) and the hash.
/// 
/// The hash is Hash(hash_left, hash_right).
/// 
/// The amount of equity and amount of debt is the sum of the equity and debt of the children.
#[derive(Debug, Copy, Clone,)]
pub struct MerkleSumNodeTarget{
    pub sum_assets: Target,
    pub sum_debt: Target,
    pub hash: HashOutTarget
}

impl MerkleSumNodeTarget {
    /// Given two children, generate the next MerkleSumNode
    pub fn get_child_from_parents(
        builder: &mut CircuitBuilder<F, D>,
        left_node: &MerkleSumNodeTarget,
        right_node: &MerkleSumNodeTarget,
    )->MerkleSumNodeTarget{
        let sum_assets = builder.add(left_node.sum_assets, right_node.sum_assets);
        let sum_debt = builder.add(left_node.sum_debt, right_node.sum_debt);

        // Ensure the amount of assets at this node is greater than the total amount of debt
        let diff_between_asset_debt = builder.sub(sum_assets, sum_debt);
        assert_non_negative_unsigned(builder, diff_between_asset_debt);

        let hash_inputs = vec![
            left_node.hash.elements.to_vec(),
            right_node.hash.elements.to_vec(),
        ].concat();

        let hash = builder.hash_n_to_hash_no_pad::<PoseidonHash>(hash_inputs);
        MerkleSumNodeTarget{
            sum_assets,
            sum_debt,
            hash
        }
    }

    /// Get a merkle sum node from account sum targets.
    pub fn get_node_from_account_targets(
        builder: &mut CircuitBuilder<F, D>,
        account_targets: &AccountSumTargets
    )-> MerkleSumNodeTarget{
        let hash_inputs = vec![
            account_targets.sum_assets,
            account_targets.sum_debt
        ];

        let hash = builder.hash_n_to_hash_no_pad::<PoseidonHash>(hash_inputs);
        MerkleSumNodeTarget{
            sum_assets: account_targets.sum_assets,
            sum_debt: account_targets.sum_debt,
            hash
        }
    }

    pub fn registered_as_public_inputs(&self, builder: &mut CircuitBuilder<F, D>) {
        builder.register_public_input(self.sum_assets);
        builder.register_public_input(self.sum_debt);
        builder.register_public_inputs(self.hash.elements.as_slice());
    }
}

impl From<Vec<Target>> for MerkleSumNodeTarget {
    /// the parsing order must be consistent with the order of public input registration in `registered_as_public_inputs`
	fn from(inputs: Vec<Target>) -> MerkleSumNodeTarget {
		let mut iter = inputs.into_iter();
		let sum_asset_target = iter.next().unwrap();
		let sum_debt_target = iter.next().unwrap();
		let hash_target = HashOutTarget::from_vec(
			iter.by_ref().take(NUM_HASH_OUT_ELTS).collect(),
		);

		MerkleSumNodeTarget {
			sum_assets: sum_asset_target,
			sum_debt: sum_debt_target,
			hash: hash_target,
		}
	}
}



/// We can represent the Merkle Sum Tree as a vector of merkle sum nodes, with the root being the last node in the vector.    
pub struct MerkleSumTreeTarget{
    pub sum_tree: Vec<MerkleSumNodeTarget>
}

impl MerkleSumTreeTarget{
    pub fn get_root(&self)-> &MerkleSumNodeTarget{
        self.sum_tree.last().unwrap()
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
        leaves.push(MerkleSumNodeTarget::get_child_from_parents(builder, left_child, right_child));
    }
}

/// Given a list of account targets, build the corresponding merkle sum tree.
pub fn build_merkle_sum_tree_from_account_targets(
    builder: &mut CircuitBuilder<F, D>,
    accounts: &mut Vec<AccountSumTargets>,
)-> MerkleSumTreeTarget {
    let mut leaves: Vec<MerkleSumNodeTarget> = accounts.iter().map( |x|
        MerkleSumNodeTarget::get_node_from_account_targets(builder, x)
    ).collect();

    build_merkle_sum_tree(builder, &mut leaves);

    let tree = MerkleSumTreeTarget{
        sum_tree: leaves
    };
    
    let root = tree.get_root();
    builder.register_public_input(root.sum_assets);
    builder.register_public_input(root.sum_debt);
    builder.register_public_inputs(&root.hash.elements);

    return tree;
}


