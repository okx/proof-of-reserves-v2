use itertools::Itertools;
use plonky2::util::log2_strict;

#[derive(Debug, Clone)]
pub struct MerkleProofIndex {
    pub sum_tree_siblings: Vec<usize>,
    pub recursive_tree_siblings: Vec<Vec<usize>>,
}
/// Get the indexes for the hashes needed for the merkle proof of inclusion, these are indexes in the global tree.
pub fn get_merkle_proof_sibling_index_for_user(
    account_index: usize,
    batch_size: usize,
    num_batches: usize,
    recursive_batch_size: usize
) -> Vec<usize> {
    let num_leaves = batch_size * num_batches;
    let batch_depth = log2_strict(batch_size);
    let mut siblings: Vec<usize> = Vec::new();
    siblings.extend(get_mst_siblings_index(account_index, batch_depth, num_leaves));

    let mst_levels = log2_strict(batch_size);
    let recursive_levels =  log2_strict(num_batches)/log2_strict(recursive_batch_size);

    for i in mst_levels+1..mst_levels+recursive_levels+1{
        let starting_level_index = get
        let recursive_siblings = get_recursive_siblings_index(account_index, batch_depth, num_leaves);
    }

    siblings
}

/// Get the sibling indexes for the merkle proof of inclusion given a leaf index.
/// We get the parent index of a leaf using the formula: parent = index / 2 + num_leaves
pub fn get_mst_siblings_index(account_index: usize, batch_depth: usize, num_leaves: usize) -> Vec<usize> {
    let mut index = account_index;
    let mut siblings: Vec<usize> = Vec::new();
    for _ in 0..batch_depth {
        if index % 2 == 1 {
            let sibling_index = index - 1;
            siblings.push(sibling_index);
        } else {
            let sibling_index = index + 1;
            siblings.push(sibling_index);
        }

        let parent = (index / 2) + num_leaves;
        index = parent;
    }

    siblings
}

/// Gets the recursive sibling nodes for a given index.
/// Note this function does not check that the index is within the level since it does not have that information.
pub fn get_recursive_siblings_index(index: usize, recursive_batch_size: usize, starting_level_idx: usize) -> Vec<u32>{
    assert!(index >= starting_level_idx);
    
    // recursive batch is which recursive batch this index belongs to. 
    let recursive_batch_no = (index-starting_level_idx) / recursive_batch_size;

    let siblings: Vec<usize> = ((starting_level_idx + recursive_batch_no * recursive_batch_size)..(starting_level_idx + (recursive_batch_no + 1) * recursive_batch_size)).collect_vec();
    siblings.iter().filter(|x| **x != index).map(|x| *x as u32).collect::<Vec<u32>>()
}   

/// Gets the index of the first node in the level;
pub fn get_first_mst_root_index(
    mut mst_batch_size: usize,
    mut num_batches: usize,
    level: usize,
    recursive_batch_size: usize,
)-> usize{
    // Check that a given level is within the calculable levels
    let levels_in_mst = log2_strict(mst_batch_size);
    let recursive_levels = log2_strict(num_batches)/log2_strict(recursive_batch_size);
    assert!(levels_in_mst + recursive_levels >= level);

    if level < levels_in_mst{
        let mut node = 0;
        for _ in 0..level{
            node += mst_batch_size*num_batches;
            mst_batch_size = mst_batch_size/2;
        }
        return node;
    }else{
        let num_nodes_in_mst = mst_batch_size*2 - 2;
        let total_num_nodes_in_mst_layers = num_nodes_in_mst*num_batches;
        let mut node  = total_num_nodes_in_mst_layers;

        for _ in 0..(level-levels_in_mst){
            node += num_batches;
            num_batches = num_batches/recursive_batch_size;
        }

        return node;
    }
}

#[cfg(test)]
pub mod test {

    use plonky2::util::log2_strict;

    use crate::merkle_proof::get_recursive_siblings_index;

    use super::{get_first_mst_root_index, get_mst_siblings_index};

    #[test]
    pub fn test_get_mst_siblings_index() {
        let batch_size = 16;
        let num_batches = 4;
        let account_index = 0;

        let sibling_indexes =
            get_mst_siblings_index(account_index, log2_strict(batch_size), batch_size*num_batches);

        let actual_siblings = vec![1, 65, 97, 113];
        assert_eq!(sibling_indexes, actual_siblings);

        let batch_size = 8;
        let num_batches = 4;
        let account_index = 1;

        let sibling_indexes =
            get_mst_siblings_index(account_index, log2_strict(batch_size), batch_size*num_batches);

        let actual_siblings = vec![0, 33, 49];
        assert_eq!(sibling_indexes, actual_siblings);
    }


    #[test]
    pub fn test_get_recursive_siblings_index() {
        let index = 48;
        let recursive_batch_size = 4;
        let starting_level_idx = 48;

        let sibling_indexes =
            get_recursive_siblings_index(index, recursive_batch_size, starting_level_idx);
        let actual_siblings = vec![49, 50, 51];
        assert_eq!(sibling_indexes, actual_siblings);

        let index = 98;
        let recursive_batch_size = 8;
        let starting_level_idx = 96;

        let sibling_indexes =
            get_recursive_siblings_index(index, recursive_batch_size, starting_level_idx);
        let actual_siblings = vec![96, 97, 99, 100, 101, 102, 103];
        assert_eq!(sibling_indexes, actual_siblings);
    }

    #[test]
    pub fn test_get_first_mst_root_index(){
        let batch_size = 16;
        let num_batches = 4;
        let recursive_batch_size = 4;

        let level = 1;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 64);

        let level = 2;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 96);

        let level = 5;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 124);


        let batch_size = 32;
        let num_batches = 16;
        let recursive_batch_size = 4;

        let level = 1;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16);

        let level = 2;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16);

        let level = 3;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16 + 8*16);

        let level = 4;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16 + 8*16 + 4*16);

        let level = 5;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16 + 8*16 + 4*16 + 2*16);

        let level = 6;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16 + 8*16 + 4*16 + 2*16 + 16);

        let level = 7;
        let starting_node_idx = get_first_mst_root_index(batch_size, num_batches, level, recursive_batch_size);
        assert_eq!(starting_node_idx, 32*16 + 16*16 + 8*16 + 4*16 + 2*16 + 16 + 4);

    }
}
