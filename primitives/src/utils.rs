use codec::{Decode, Encode};
use sp_runtime::traits::AtLeast32Bit;
use sp_std::vec::Vec;

use crate::types::ResourceId;

pub fn compute_chain_id_type<ChainId>(chain_id: ChainId, chain_type: [u8; 2]) -> u64
where
	ChainId: AtLeast32Bit,
{
	let mut chain_id_value: u32 = chain_id.try_into().unwrap_or_default();
	let mut buf = [0u8; 8];
	buf[2..4].copy_from_slice(&chain_type);
	buf[4..8].copy_from_slice(&chain_id_value.to_be_bytes());
	u64::from_be_bytes(buf)
}

/// The ResourceId type is a 32 bytes array represented as the following:
/// ```md
/// +---+---+---+---+---+---+---+---+---+
/// | * |   |   |  ...  | * | * | * | * |
/// +-|-+---+---+---+---+-|-+---+---+---+
///   |                   +-> The last 4 bytes are the chain_id
///   +-> The first 20 bytes are the tree_id
/// ```
/// This takes the tree_id and the chain_id and combines them into a single 32
/// bytes array. the process is simple as convert the `tree_id` to 4 bytes array
/// (little-endian), pad the remaining `(20 - 4)` butes with zeros, next convert
/// the chain_id to 4 bytes array (little-endian) and append it to the last 4
/// bytes of the result array.
pub fn encode_resource_id<TreeId, ChainId>(tree_id: TreeId, chain_id: ChainId) -> ResourceId
where
	TreeId: Encode + Decode + AtLeast32Bit + Default + Copy,
	ChainId: Encode + Decode + AtLeast32Bit + Default + Copy,
{
	let mut result = [0u8; 32];
	let mut tree_id_bytes = tree_id.encode();
	tree_id_bytes.resize(20, 0); // fill the remaining 20 bytes with zeros
	let mut chain_id_bytes = chain_id.encode();
	chain_id_bytes.resize(4, 0); // fill the remaining 4 bytes with zeros

	debug_assert!(tree_id_bytes.len() == 20);
	debug_assert!(chain_id_bytes.len() == 4);
	result[0..20].copy_from_slice(&tree_id_bytes);
	result[28..].copy_from_slice(&chain_id_bytes);
	result
}

/// The ResourceId type is a 32 bytes array represented as the following:
/// ```md
/// +---+---+---+---+---+---+---+---+---+
/// | * |   |   |  ...  | * | * | * | * |
/// +-|-+---+---+---+---+-|-+---+---+---+
///   |                   +-> The last 4 bytes are the chain_id
///   +-> The first 20 bytes are the tree_id
/// ```
/// This takes the resource_id and returns the tree_id and the chain_id.
/// The process is fairly simple (it the reverse of the [`encode_resource_id`])
/// as we read the first 20 bytes of the resource_id as the `tree_id` and the
/// last 4 bytes of the resource_id as `chain_id`.
pub fn decode_resource_id<TreeId, ChainId>(resource_id: ResourceId) -> (TreeId, ChainId)
where
	TreeId: Encode + Decode + AtLeast32Bit + Default + Copy,
	ChainId: Encode + Decode + AtLeast32Bit + Default + Copy,
{
	let tree_id_bytes = &resource_id[0..20];
	let chain_id_bytes = &resource_id[28..];
	let tree_id = TreeId::decode(&mut &*tree_id_bytes).unwrap();
	let chain_id = ChainId::decode(&mut &*chain_id_bytes).unwrap();
	(tree_id, chain_id)
}

/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}

pub fn element_encoder(v: &[u8]) -> [u8; 32] {
	let mut output = [0u8; 32];
	output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
	output
}

#[cfg(test)]
mod tests {
	use super::*;
	type TreeId = u32;
	type ChainId = u64;

	#[test]
	fn encode_decode_resource_ids() {
		let tree_id = 1;
		let chain_id = 2000;
		let resource_id = encode_resource_id::<TreeId, ChainId>(tree_id, chain_id);
		let (tree_id2, chain_id2) = decode_resource_id::<TreeId, ChainId>(resource_id);
		assert_eq!(tree_id, tree_id2);
		assert_eq!(chain_id, chain_id2);
	}
}
