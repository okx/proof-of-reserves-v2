use crate::types::F;
use plonky2::hash::hash_types::HashOut;
use plonky2_field::types::Field;

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

/// Given a hash string, get a hashout
pub fn get_hash_from_hash_string(hash_string: String) -> HashOut<F> {
    let without_brackets = hash_string.trim_matches(|c| c == '[' || c == ']').to_string(); // Remove brackets

    let hash_as_vec_f: Vec<F> = without_brackets
        .split(',')
        .map(|s| F::from_canonical_u64(s.parse::<u64>().unwrap()))
        .collect();

    if hash_as_vec_f.len() != 4 {
        panic!("Incorrect format of hash");
    }

    HashOut::from_vec(hash_as_vec_f)
}

#[cfg(test)]
pub mod test_util {
    use plonky2::hash::hash_types::HashOut;

    use crate::util::get_node_level;

    use super::{get_hash_from_hash_string, pad_to_multiple_of};

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

    #[test]
    fn test_get_hash_from_hash_string() {
        let hash = get_hash_from_hash_string("[0000,0000,0000,0000]".to_string());
        assert_eq!(hash, HashOut::ZERO);
    }
}
