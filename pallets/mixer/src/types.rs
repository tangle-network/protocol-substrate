//! All the traits exposed to be used in other custom pallets
use crate::*;
use frame_support::dispatch;
use codec::{Encode, Decode};

/// Tree trait definition to be used in other pallets
pub trait MixerInterface<T: Config<I>, I: 'static = ()> {
	/// Deposit into the mixer
	fn deposit(account: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), dispatch::DispatchError>;
	/// Withdraw into the mixer
	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
	) -> Result<(), dispatch::DispatchError>;
}

#[derive(Default, Clone, Encode, Decode)]
pub struct MixerMetadata<AccountId, Balance> {
	/// Creator account
	pub creator: AccountId,
	/// Balance size of deposit
	pub deposit_size: Balance,
}