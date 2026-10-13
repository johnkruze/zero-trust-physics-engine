//! The row tree: the Merkle tree of RFC 6962.
//!
//! A leaf hash is SHA-256 of the byte 0x00 followed by the leaf. A node hash
//! is SHA-256 of the byte 0x01 followed by the left hash and the right hash.
//! A tree of more than one leaf splits at the largest power of two below its
//! count. A leaf is never copied to fill a level, so a list of leaves and the
//! same list with its last leaf repeated have different roots.

use sha2::{Digest, Sha256};

pub type Hash = [u8; 32];

/// SHA-256 of 0x00 followed by `leaf`.
pub fn leaf_hash(leaf: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x00_u8]);
    hasher.update(leaf);
    finish(hasher)
}

/// SHA-256 of 0x01 followed by `left` and `right`.
pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x01_u8]);
    hasher.update(left);
    hasher.update(right);
    finish(hasher)
}

fn finish(hasher: Sha256) -> Hash {
    let mut out = [0_u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// The largest power of two below `count`. `count` is at least 2.
fn split(count: u64) -> u64 {
    let mut power = 1_u64;
    while power <= (count - 1) / 2 {
        power *= 2;
    }
    power
}

/// The root over `leaves`, a list of leaf hashes in order. An empty list has
/// no root.
pub fn root(leaves: &[Hash]) -> Option<Hash> {
    match leaves.len() {
        0 => None,
        1 => Some(leaves[0]),
        count => {
            let at = split(count as u64) as usize;
            Some(node_hash(&root(&leaves[..at])?, &root(&leaves[at..])?))
        }
    }
}

/// The audit path of leaf `index`: the sibling hashes from the leaf up to the
/// root, lowest first.
pub fn audit_path(leaves: &[Hash], index: u64) -> Option<Vec<Hash>> {
    let count = leaves.len() as u64;
    if index >= count {
        return None;
    }
    if count == 1 {
        return Some(Vec::new());
    }
    let at = split(count);
    let (left, right) = leaves.split_at(at as usize);
    if index < at {
        let mut path = audit_path(left, index)?;
        path.push(root(right)?);
        Some(path)
    } else {
        let mut path = audit_path(right, index - at)?;
        path.push(root(left)?);
        Some(path)
    }
}

/// The root that `leaf` at `index` reaches in a tree of `count` leaves along
/// `path`. Nothing is returned when the index is not in the tree or the path
/// is not the length that leaf has in that tree.
pub fn root_from_path(index: u64, count: u64, leaf: &Hash, path: &[Hash]) -> Option<Hash> {
    if index >= count {
        return None;
    }
    if count == 1 {
        return if path.is_empty() { Some(*leaf) } else { None };
    }
    let (sibling, lower) = path.split_last()?;
    let at = split(count);
    if index < at {
        let left = root_from_path(index, at, leaf, lower)?;
        Some(node_hash(&left, sibling))
    } else {
        let right = root_from_path(index - at, count - at, leaf, lower)?;
        Some(node_hash(sibling, &right))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Roots worked out by `outside_check.py` from RFC 6962, for the leaves
    /// `leaf 0`, `leaf 1` and so on as ASCII text.
    const VECTORS: &str = include_str!("../vectors/tree.txt");

    fn leaves(count: usize) -> Vec<Hash> {
        (0..count)
            .map(|index| leaf_hash(format!("leaf {index}").as_bytes()))
            .collect()
    }

    fn published(kind: &str, count: usize) -> String {
        for line in VECTORS.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 3 && parts[0] == kind && parts[1] == count.to_string() {
                return parts[2].to_string();
            }
        }
        panic!("no published {kind} for {count} leaves");
    }

    #[test]
    fn small_trees_have_the_roots_worked_out_outside_this_crate() {
        for count in [1, 2, 3, 5, 7, 8] {
            assert_eq!(
                hex::encode(root(&leaves(count)).unwrap()),
                published("root", count),
                "{count} leaves"
            );
        }
        assert!(root(&[]).is_none());
    }

    #[test]
    fn the_empty_leaf_and_the_split_follow_rfc_6962() {
        // SHA-256 of the single byte 0x00, the leaf hash of an empty leaf.
        assert_eq!(
            hex::encode(leaf_hash(b"")),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
        for (count, at) in [(2, 1), (3, 2), (4, 2), (5, 4), (7, 4), (8, 4), (9, 8), (100_000, 65_536)] {
            assert_eq!(split(count), at, "{count}");
        }
        assert_eq!(split(u64::MAX), 1 << 63);
        let five = leaves(5);
        let left = node_hash(&node_hash(&five[0], &five[1]), &node_hash(&five[2], &five[3]));
        assert_eq!(root(&five).unwrap(), node_hash(&left, &five[4]));
    }

    #[test]
    fn every_leaf_of_a_small_tree_reaches_the_root_along_its_audit_path() {
        for count in [1, 2, 3, 5, 7, 8, 13] {
            let all = leaves(count);
            let top = root(&all).unwrap();
            for index in 0..count as u64 {
                let path = audit_path(&all, index).unwrap();
                assert_eq!(
                    root_from_path(index, count as u64, &all[index as usize], &path),
                    Some(top),
                    "leaf {index} of {count}"
                );
            }
            assert!(audit_path(&all, count as u64).is_none());
        }
    }

    #[test]
    fn the_published_audit_path_of_leaf_4_of_7_is_reproduced() {
        let all = leaves(7);
        let path: Vec<String> = audit_path(&all, 4).unwrap().iter().map(hex::encode).collect();
        let mut expected = Vec::new();
        for line in VECTORS.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "path_7_4" {
                expected.push(parts[1].to_string());
            }
        }
        assert_eq!(expected.len(), 3);
        assert_eq!(path, expected);
    }

    #[test]
    fn two_leaves_swapped_change_the_root() {
        for count in [2, 3, 5, 8] {
            let all = leaves(count);
            let mut swapped = all.clone();
            swapped.swap(0, count - 1);
            assert_ne!(root(&all), root(&swapped), "{count} leaves");
        }
    }

    #[test]
    fn a_leaf_dropped_changes_the_root() {
        let all = leaves(8);
        for dropped in 0..8 {
            let mut fewer = all.clone();
            fewer.remove(dropped);
            assert_ne!(root(&all), root(&fewer), "leaf {dropped}");
        }
    }

    #[test]
    fn a_leaf_repeated_changes_the_root() {
        let all = leaves(5);
        let mut repeated = all.clone();
        repeated.insert(2, all[2]);
        assert_ne!(root(&all), root(&repeated));
    }

    /// A tree that fills an odd level by copying its last hash, as some
    /// ledgers do. It is here to show the fault RFC 6962 avoids.
    fn root_that_copies_the_last_hash(leaves: &[Hash]) -> Hash {
        let mut level = leaves.to_vec();
        while level.len() > 1 {
            if level.len() % 2 == 1 {
                level.push(level[level.len() - 1]);
            }
            level = level
                .chunks(2)
                .map(|pair| node_hash(&pair[0], &pair[1]))
                .collect();
        }
        level[0]
    }

    #[test]
    fn doubling_the_last_leaf_changes_the_root_here_and_not_in_a_tree_that_copies() {
        for count in [1, 3, 5, 7] {
            let all = leaves(count);
            let mut doubled = all.clone();
            doubled.push(all[count - 1]);
            // The copying tree cannot tell the two lists apart.
            if count > 1 {
                assert_eq!(
                    root_that_copies_the_last_hash(&all),
                    root_that_copies_the_last_hash(&doubled),
                    "{count} leaves"
                );
            }
            // The RFC 6962 tree can.
            assert_ne!(root(&all), root(&doubled), "{count} leaves");
        }
    }

    #[test]
    fn a_leaf_is_not_a_node() {
        // The 64 bytes of two child hashes, offered as one leaf, do not hash
        // to their parent: the leaf prefix and the node prefix differ.
        let all = leaves(2);
        let mut joined = Vec::new();
        joined.extend_from_slice(&all[0]);
        joined.extend_from_slice(&all[1]);
        assert_ne!(leaf_hash(&joined), root(&all).unwrap());
    }

    #[test]
    fn a_path_for_one_leaf_does_not_carry_its_neighbour() {
        for count in [2, 3, 5, 7, 8] {
            let all = leaves(count);
            let top = root(&all).unwrap();
            for index in 0..count as u64 {
                let path = audit_path(&all, index).unwrap();
                for other in 0..count as u64 {
                    if other == index {
                        continue;
                    }
                    // The neighbour's leaf at this leaf's place.
                    assert_ne!(
                        root_from_path(index, count as u64, &all[other as usize], &path),
                        Some(top)
                    );
                    // This leaf's path at the neighbour's place.
                    assert_ne!(
                        root_from_path(other, count as u64, &all[other as usize], &path),
                        Some(top)
                    );
                }
            }
        }
    }

    #[test]
    fn a_path_of_the_wrong_length_or_for_another_count_is_not_followed() {
        let all = leaves(7);
        let top = root(&all).unwrap();
        let path = audit_path(&all, 6).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(root_from_path(6, 7, &all[6], &path), Some(top));
        assert_eq!(root_from_path(6, 7, &all[6], &path[..1]), None);
        let mut longer = path.clone();
        longer.push(all[0]);
        assert_eq!(root_from_path(6, 7, &all[6], &longer), None);
        assert_ne!(root_from_path(6, 8, &all[6], &path), Some(top));
        assert_eq!(root_from_path(7, 7, &all[6], &path), None);
    }
}
