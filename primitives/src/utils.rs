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
	let range = if id.len() > 26 { 26 } else { id.len() }; // Use at most 26 bytes
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
	fn derive_parse_resource_ids() {
		let tree_id = 1u32;
		let chain_id = 2000u32;
		let updated_chain_id = compute_chain_id_type(chain_id, [2, 0]);
		let resource_id = derive_resource_id(updated_chain_id, &tree_id.encode());
		let (tree_id2, chain_id2) = parse_resource_id(resource_id);
		assert_eq!(tree_id, tree_id2);
		assert_eq!(updated_chain_id, chain_id2);
	}
}
