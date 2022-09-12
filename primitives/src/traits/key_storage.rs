//! All the traits exposed to be used in other custom pallets
use frame_support::dispatch;
use sp_std::vec::Vec;

pub trait KeyStorageConfig {
	type AccountId;
}

/// KeyStorage trait definition to be used in other pallets
pub trait KeyStorageInterface<C: KeyStorageConfig> {
	/// Registers a new public key to the owner
	fn register(owner: C::AccountId, key: Vec<u8>) -> Result<(), dispatch::DispatchError>;
}
