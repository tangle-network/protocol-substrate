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
//!
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::convert::TryInto;
use frame_support::{ pallet_prelude::DispatchError};
use sp_std::prelude::*;
use webb_primitives::traits::key_storage::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;
	}

	/// The map of owners to public keys
	#[pallet::storage]
	#[pallet::getter(fn public_key_owners)]
	pub type PublicKeyOwners<T: Config<I>, I: 'static = ()> =
	StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u8>, ValueQuery>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New tree created
		PublicKeyRegisteration { owner: T::AccountId, public_key: Vec<u8> },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(50_000_000)]
		pub fn register(
			origin: OriginFor<T>,
			owner: T::AccountId,
			public_key: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<Self as KeyStorageInterface<_>>::register(owner.clone(), public_key.clone())?;
			Self::deposit_event(Event::PublicKeyRegisteration { owner, public_key });
			Ok(().into())
		}
	}

}

pub struct KeyStorageConfiguration<T: Config<I>, I: 'static>(
	core::marker::PhantomData<T>,
	core::marker::PhantomData<I>,
);

impl<T: Config<I>, I: 'static> KeyStorageConfig for KeyStorageConfiguration<T, I> {
	type AccountId = T::AccountId;
}

impl<T: Config<I>, I: 'static> KeyStorageInterface<KeyStorageConfiguration<T, I>>
for Pallet<T, I>
{
	fn register(
		owner: T::AccountId,
		public_key: Vec<u8>,
	) -> Result<(), DispatchError> {
		PublicKeyOwners::<T, I>::insert(owner.clone(), public_key.clone());
		#[cfg(feature = "std")]
			{
				println!("Registered public key with owner: {:?}, {:?}", owner, public_key);
			}
		Ok(())
	}
}
