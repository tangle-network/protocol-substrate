//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use frame_support::dispatch;

/// Tree trait definition to be used in other pallets
pub trait AnchorInterface<T: Config<I>, I: 'static = ()> {
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

/// Tree trait for inspecting tree state
pub trait AnchorInspector<T: Config<I>, I: 'static = ()> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_neighbor_roots(id: T::TreeId) -> Result<T::Element, dispatch::DispatchError>;	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_neighbor_root(id: T::TreeId, target: T::Element) -> Result<bool, dispatch::DispatchError>;
}

#[derive(Default, Clone, Encode, Decode)]
pub struct AnchorMetadata<AccountId, Balance> {
	/// Creator account
	pub creator: AccountId,
	/// Balance size of deposit
	pub deposit_size: Balance,
}
