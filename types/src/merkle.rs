use crate::Hash;
use alloc::vec::Vec;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::MerkleHash;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct MerklePathItem {
    pub hash: MerkleHash,
    pub direction: Direction,
}

pub type MerklePath = Vec<MerklePathItem>;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
}

pub fn combine_hash(hash1: &MerkleHash, hash2: &MerkleHash) -> MerkleHash {
    hash_borsh((hash1, hash2))
}

pub fn verify_hash<'a>(
    root: MerkleHash,
    path: impl Iterator<Item = &'a MerklePathItem>,
    item_hash: MerkleHash,
) -> bool {
    compute_root_from_path(path, item_hash) == root
}

pub fn compute_root_from_path<'a>(
    path: impl Iterator<Item = &'a MerklePathItem>,
    item_hash: MerkleHash,
) -> MerkleHash {
    let mut hash_so_far = item_hash;
    for uncle in path {
        match uncle.direction {
            Direction::Left => {
                hash_so_far = combine_hash(&uncle.hash, &hash_so_far);
            }
            Direction::Right => {
                hash_so_far = combine_hash(&hash_so_far, &uncle.hash);
            }
        }
    }
    hash_so_far
}

pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = sha2::Sha256::default();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn hash_borsh<T>(value: T) -> Hash {
    todo!("provide intobytes from host")
    // let mut hasher = sha2::Sha256::default();
    // let mut bytes = Vec::new();
    // value.serialize(&mut bytes).unwrap();
    // hasher.update(bytes);
    // hasher.finalize().into()
}

pub fn compute_root_from_path_and_item<'a, T>(
    path: impl Iterator<Item = &'a MerklePathItem>,
    item: T,
) -> MerkleHash {
    compute_root_from_path(path, hash_borsh(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        todo!()
    }
}
