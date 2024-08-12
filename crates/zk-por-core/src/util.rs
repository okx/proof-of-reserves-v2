/// compute total number of hashes for recursive trees
pub fn get_recursive_hash_nums(num_of_batches: usize, hyper_tree_leaf_size: usize) -> usize {
    assert!(num_of_batches > 0);
    if num_of_batches <= hyper_tree_leaf_size {
        return 1;
    }

    let mut next = num_of_batches.div_ceil(hyper_tree_leaf_size);

    let mut num_of_hashes =
        pad_to_multiple_of(num_of_batches, hyper_tree_leaf_size) - num_of_batches;
    while next > 1 {
        next = next.div_ceil(hyper_tree_leaf_size);
        num_of_hashes += next * hyper_tree_leaf_size;
    }
    num_of_hashes + 1
}

pub fn pad_to_multiple_of(n: usize, multiple: usize) -> usize {
    if multiple == 0 {
        return n; // Avoid division by zero
    }
    let remainder = n % multiple;
    if remainder == 0 {
        n
    } else {
        n + multiple - remainder
    }
}

/// node level is the level from tree root; the root node has level of 0;
/// `node_idx` is the index of the nodes in a vector; the root node has the largest ndoe_idx
pub fn get_node_level(batch_size: usize, node_idx: usize) -> usize {
    let total_nums = 2 * batch_size - 1;
    ((total_nums - node_idx) as f64).log(2.0).floor() as usize
}

#[cfg(test)]
pub mod test_util {
    use crate::util::{get_node_level, get_recursive_hash_nums};

    use super::pad_to_multiple_of;

    #[test]
    fn test_get_recursive_hash_nums() {
        assert_eq!(get_recursive_hash_nums(2, 4), 1);
        assert_eq!(get_recursive_hash_nums(4, 4), 1);
        assert_eq!(get_recursive_hash_nums(6, 4), 7);
        assert_eq!(get_recursive_hash_nums(100, 4), 41);
    }
    #[test]
    fn test_get_node_level() {
        assert_eq!(get_node_level(8, 14), 0);
        assert_eq!(get_node_level(8, 13), 1);
        assert_eq!(get_node_level(8, 9), 2);
        assert_eq!(get_node_level(8, 11), 2);
        assert_eq!(get_node_level(8, 0), 3);
        assert_eq!(get_node_level(8, 4), 3);
        assert_eq!(get_node_level(8, 7), 3);
    }

    #[test]
    fn test_pad_to_multiple_of() {
        assert_eq!(pad_to_multiple_of(23, 4), 24);
        assert_eq!(pad_to_multiple_of(24, 4), 24);
        assert_eq!(pad_to_multiple_of(27, 4), 28);
    }
}
