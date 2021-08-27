use crate::types::ResourceId;

/// Helper function to concatenate a chain ID and some bytes to produce a
/// resource ID. The common format is (31 bytes unique ID + 1 byte chain ID).
pub fn derive_resource_id(chain: u32, id: &[u8]) -> ResourceId {
	let mut r_id: ResourceId = [0; 32];
	r_id[31] = chain.to_le_bytes()[0]; // last byte of chain id, just for testing
	let range = if id.len() > 31 { 31 } else { id.len() }; // Use at most 31 bytes
	for i in 0..range {
		r_id[30 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
	}
	return r_id;
}
