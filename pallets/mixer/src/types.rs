//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use frame_support::{dispatch, ensure};
use scale_info::TypeInfo;

/// Mixer trait definition to be used in other pallets
pub trait MixerInterface<T: Config<I>, I: 'static = ()> {
	// Creates a new mixer
	fn create(
		creator: T::AccountId,
		deposit_size: BalanceOf<T, I>,
		depth: u8,
	) -> Result<T::TreeId, dispatch::DispatchError>;
	/// Deposit into the mixer
	fn deposit(account: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), dispatch::DispatchError>;
	/// Withdraw from the mixer
	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		root: T::Element,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), dispatch::DispatchError>;
	// Stores nullifier hash from a spend tx
	fn add_nullifier_hash(id: T::TreeId, nullifier_hash: T::Element) -> Result<(), dispatch::DispatchError>;
}

/// Mixer trait for inspecting mixer state
pub trait MixerInspector<T: Config<I>, I: 'static = ()> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_root(id: T::TreeId) -> Result<T::Element, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_root(id: T::TreeId, target: T::Element) -> Result<bool, dispatch::DispatchError>;
	fn ensure_known_root(id: T::TreeId, target: T::Element) -> Result<(), dispatch::DispatchError> {
		let is_known: bool = Self::is_known_root(id, target)?;
		ensure!(is_known, Error::<T, I>::InvalidWithdrawRoot);
		Ok(())
	}
	/// Check if a nullifier has been used in a tree or returns
	/// `InvalidNullifier`
	fn is_nullifier_used(id: T::TreeId, nullifier: T::Element) -> bool;
	fn ensure_nullifier_unused(id: T::TreeId, nullifier: T::Element) -> Result<(), dispatch::DispatchError> {
		ensure!(
			Self::is_nullifier_used(id, nullifier),
			Error::<T, I>::AlreadyRevealedNullifier
		);
		Ok(())
	}
}

#[derive(Default, Clone, Encode, Decode, TypeInfo)]
pub struct MixerMetadata<AccountId, Balance> {
	/// Creator account
	pub creator: AccountId,
	/// Balance size of deposit
	pub deposit_size: Balance,
}
