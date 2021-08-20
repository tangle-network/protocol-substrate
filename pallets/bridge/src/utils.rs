use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResult,
	ensure,
	traits::{EnsureOrigin, Get},
	weights::{GetDispatchInfo, Pays},
	Parameter,
};

use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
	traits::{AccountIdConversion, Dispatchable},
	ModuleId, RuntimeDebug,
};
use sp_std::prelude::*;

use codec::{Decode, Encode, EncodeLike};

/// Helper function to concatenate a chain ID and some bytes to produce a
/// resource ID. The common format is (31 bytes unique ID + 1 byte chain ID).
pub fn derive_resource_id(chain: u8, id: &[u8]) -> ResourceId {
	let mut r_id: ResourceId = [0; 32];
	r_id[31] = chain; // last byte is chain id
	let range = if id.len() > 31 { 31 } else { id.len() }; // Use at most 31 bytes
	for i in 0..range {
		r_id[30 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
	}
	return r_id;
}
