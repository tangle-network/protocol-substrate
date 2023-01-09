//! # Key-Storage Module
//!
//! A module for storing public keys.
//!
//! ## Overview
//!
//! The Key-storage module provides functionality for the following:
//!
//! * Registering new public keys
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Goals
//!
//! The Key-storage in Webb is designed to make the following possible:
//!
//! * Store public key of a particular substrate address
//!
//! ## KeyStorageInterface Interface
//!
//! `register`: Registers a public key to it's account.
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
pub mod tests;
pub mod weights;

use frame_support::{pallet_prelude::DispatchError, BoundedVec};
use sp_std::{convert::TryInto, prelude::*};
use std::convert::TryFrom;
use webb_primitives::traits::key_storage::*;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]

	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The max length accepted for Pubkey
		type MaxPubkeyLength: Get<u32>;

		/// The max number of pubkey owneres we can store
		type MaxPubKeyOwners: Get<u32>;

		/// Weightinfo for pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: (PhantomData<T>, PhantomData<I>),
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {}
	}

	/// The map of owners to public keys
	#[pallet::storage]
	#[pallet::getter(fn public_key_owners)]
	pub type PublicKeyOwners<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		BoundedVec<(T::AccountId, BoundedVec<u8, T::MaxPubkeyLength>), T::MaxPubKeyOwners>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Public key registration
		PublicKeyRegistration {
			owner: T::AccountId,
			public_key: BoundedVec<u8, T::MaxPubkeyLength>,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		PubKeyOutOfBounds,
		MaxPubkeyOwnersExceeded,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::register(public_key.len() as u32))]
		pub fn register(
			origin: OriginFor<T>,
			owner: T::AccountId,
			public_key: BoundedVec<u8, T::MaxPubkeyLength>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			<Self as KeyStorageInterface<_>>::register(
				owner.clone(),
				public_key.clone().into_inner(),
			)?;
			Self::deposit_event(Event::PublicKeyRegistration { owner, public_key });
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> KeyStorageInterface<T::AccountId> for Pallet<T, I> {
	fn register(owner: T::AccountId, public_key: Vec<u8>) -> Result<(), DispatchError> {
		let bounded_public_key = BoundedVec::<u8, T::MaxPubkeyLength>::try_from(public_key.clone())
			.map_err(|e| Error::<T, I>::PubKeyOutOfBounds)?;
		let mut public_key_owners = <PublicKeyOwners<T, I>>::get();
		public_key_owners
			.try_push((owner.clone(), bounded_public_key))
			.map_err(|e| Error::<T, I>::MaxPubkeyOwnersExceeded)?;
		#[cfg(feature = "std")]
		{
			println!("Registered public key with owner: {owner:?}, {public_key:?}");
		}
		Ok(())
	}
}
