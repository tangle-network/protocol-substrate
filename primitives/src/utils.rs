use codec::{Decode, Encode};
use scale_info::prelude::fmt::Debug;
use sp_runtime::traits::AtLeast32Bit;
use sp_std::vec::Vec;

use crate::types::ResourceId;

/// Takes an (ideally u32) ChainIdentifier and a chain type and re-computes
/// an updated chain id with the chain type prepended to it. The resulting
/// chain id is 6 bytes long and so requires a u64 to represent it.
///
/// ```rust
/// pub type ChainId = u64;
/// let chain_id: u32 = 5;
/// let chain_type: [u8; 2] = [2, 0];
///
/// let chain_id_type: ChainId = compute_chain_id_type(chain_id.into(), chain_type);
/// ```
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

/// Helper function to concatenate a chain ID and some bytes to produce a
/// resource ID. The common format is (31 bytes unique ID + 1 byte chain ID).
pub fn derive_resource_id(chain: u64, id: &[u8]) -> ResourceId {
	let mut r_id: ResourceId = [0; 32];
	let chain = chain.to_be_bytes();
	// last 6 bytes of chain id because chain[0] and chain[1] are 0.
	r_id[26] = chain[2];
	r_id[27] = chain[3];
	r_id[28] = chain[4];
	r_id[29] = chain[5];
	r_id[30] = chain[6];
	r_id[31] = chain[7];
	let range = if id.len() > 26 { 26 } else { id.len() }; // Use at most 28 bytes
	for i in 0..range {
		r_id[25 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
	}
	r_id
}

pub fn parse_resource_id<TreeId, ChainId>(resource_id: ResourceId) -> (TreeId, ChainId)
where
	TreeId: Encode + Decode + AtLeast32Bit + Default + Copy,
	ChainId: Encode + Decode + AtLeast32Bit + Default + Copy,
{
	let tree_id_bytes = &resource_id[6..26];
	let mut chain_id_bytes = [0u8; 8];
	chain_id_bytes[2..8].copy_from_slice(&resource_id[26..]);
	let tree_id = TreeId::decode(&mut &*tree_id_bytes).unwrap();
	let chain_id = ChainId::try_from(u64::from_be_bytes(chain_id_bytes)).unwrap_or_default();
	(tree_id, chain_id)
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
///
/// TODO: Get rid of this method and use `derive_resource_id` instead.
/// NOTE: chain_id type is meant to be 6 bytes long and at the end of the resource ID byte array.
pub fn encode_resource_id<TreeId, ChainId>(tree_id: TreeId, chain_id: ChainId) -> ResourceId
where
	TreeId: Encode + Decode + AtLeast32Bit + Default + Copy + Debug,
	ChainId: Encode + Decode + AtLeast32Bit + Default + Copy + Debug,
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
///
/// TODO: Update this method as it is not consistent with `derive_resource_id` function above.
/// NOTE: chain_id type is meant to be 6 bytes long and at the end of the resource ID byte array.
/// NOTE: tree_id type is meant to be encoded into the adjacent 20 bytes to the encoded chain_id.
pub fn decode_resource_id<TreeId, ChainId>(resource_id: ResourceId) -> (TreeId, ChainId)
where
	TreeId: Encode + Decode + AtLeast32Bit + Default + Copy,
	ChainId: Encode + Decode + AtLeast32Bit + Default + Copy,
{
	let tree_id_bytes = &resource_id[0..20];
	let chain_id_bytes = &resource_id[28..];
	let mut buf_u64: [u8; 8] = [0u8; 8];
	buf_u64[0..4].copy_from_slice(chain_id_bytes);
	let tree_id = TreeId::decode(&mut &*tree_id_bytes).unwrap();
	let chain_id = ChainId::try_from(u64::from_le_bytes(buf_u64)).unwrap_or_default();
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
