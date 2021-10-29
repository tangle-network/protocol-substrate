// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Mixer Module
//!
//! A simple module for building Mixers.
//!
//! ## Overview
//!
//! The Mixer module provides functionality for SMT operations
//! including:
//!
//! * Inserting elements to the tree
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The Mixer system in Webb is designed to make the following possible:
//!
//! * Define.
//!
//! ## Interface
//!
//! ## Related Modules
//!
//! * [`System`](../frame_system/index.html)
//! * [`Support`](../frame_support/index.html)

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod zk_config;

mod benchmarking;

pub mod types;
pub mod weights;
use types::MixerMetadata;

use codec::Encode;
use darkwebb_primitives::{
	traits::{
		merkle_tree::{TreeInspector, TreeInterface},
		mixer::{MixerInspector, MixerInterface},
	},
	verifier::*,
};
use frame_support::{
	ensure, pallet_prelude::DispatchError, sp_runtime::traits::AccountIdConversion, traits::Get, PalletId,
};
use orml_traits::MultiCurrency;
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> =
	<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

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
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_mt::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The tree
		type Tree: TreeInterface<Self::AccountId, Self::TreeId, Self::Element>
			+ TreeInspector<Self::AccountId, Self::TreeId, Self::Element>;

		/// The verifier
		type Verifier: VerifierModule;

		/// Currency type for taking deposits
		type Currency: MultiCurrency<Self::AccountId>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		/// WeightInfo for pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their mixer metadata
	#[pallet::storage]
	#[pallet::getter(fn mixers)]
	pub type Mixers<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Option<MixerMetadata<T::AccountId, BalanceOf<T, I>, CurrencyIdOf<T, I>>>,
		ValueQuery,
	>;

	/// The map of trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn nullifier_hashes)]
	pub type NullifierHashes<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::TreeId, Blake2_128Concat, T::Element, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet {
			old_maintainer: T::AccountId,
			new_maintainer: T::AccountId,
		},
		/// New tree created
		MixerCreation { tree_id: T::TreeId },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
		/// Invalid nullifier that is already used
		/// (this error is returned when a nullifier is used twice)
		AlreadyRevealedNullifier,
		/// Invalid root used in withdrawal
		InvalidWithdrawRoot,
		/// No mixer found
		NoMixerFound,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(<T as Config<I>>::WeightInfo::create(*depth as u32))]
		pub fn create(
			origin: OriginFor<T>,
			deposit_size: BalanceOf<T, I>,
			depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id =
				<Self as MixerInterface<_, _, _, _, _>>::create(T::AccountId::default(), deposit_size, depth, asset)?;
			Self::deposit_event(Event::MixerCreation { tree_id });
			Ok(().into())
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::set_maintainer())]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet {
					old_maintainer: origin,
					new_maintainer,
				});
				Ok(().into())
			})
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::force_set_maintainer())]
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet {
					old_maintainer: Default::default(),
					new_maintainer,
				});
				Ok(().into())
			})
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::deposit())]
		pub fn deposit(origin: OriginFor<T>, tree_id: T::TreeId, leaf: T::Element) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as MixerInterface<_, _, _, _, _>>::deposit(origin, tree_id, leaf)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			id: T::TreeId,
			proof_bytes: Vec<u8>,
			root: T::Element,
			nullifier_hash: T::Element,
			recipient: T::AccountId,
			relayer: T::AccountId,
			fee: BalanceOf<T, I>,
			refund: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			<Self as MixerInterface<_, _, _, _, _>>::withdraw(
				id,
				&proof_bytes,
				root,
				nullifier_hash,
				recipient,
				relayer,
				fee,
				refund,
			)?;
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> MixerInterface<T::AccountId, BalanceOf<T, I>, CurrencyIdOf<T, I>, T::TreeId, T::Element>
	for Pallet<T, I>
{
	fn create(
		creator: T::AccountId,
		deposit_size: BalanceOf<T, I>,
		depth: u8,
		asset: CurrencyIdOf<T, I>,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::Tree::create(creator.clone(), depth)?;
		Mixers::<T, I>::insert(
			id,
			Some(MixerMetadata {
				creator,
				deposit_size,
				asset,
			}),
		);
		Ok(id)
	}

	fn deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), DispatchError> {
		// insert the leaf
		T::Tree::insert_in_order(id, leaf)?;

		let mixer = Self::get_mixer(id)?;
		// transfer tokens to the pallet
		<T as pallet::Config<I>>::Currency::transfer(mixer.asset, &depositor, &Self::account_id(), mixer.deposit_size)?;

		Ok(())
	}

	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		root: T::Element,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		let mixer = Self::get_mixer(id)?;
		// Check if local root is known
		ensure!(T::Tree::is_known_root(id, root)?, Error::<T, I>::InvalidWithdrawRoot);
		// Check nullifier and add or return `AlreadyRevealedNullifier`
		Self::ensure_nullifier_unused(id, nullifier_hash)?;
		Self::add_nullifier_hash(id, nullifier_hash)?;
		// Format proof public inputs for verification
		// FIXME: This is for a specfic gadget so we ought to create a generic handler
		// FIXME: Such as a unpack/pack public inputs trait
		// FIXME: 	-> T::PublicInputTrait::validate(public_bytes: &[u8])
		let mut bytes = vec![];
		let element_encoder = |v: &[u8]| {
			let mut output = [0u8; 32];
			output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
			output
		};
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);
		let fee_bytes = fee.using_encoded(element_encoder);
		let refund_bytes = refund.using_encoded(element_encoder);
		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&root.encode());
		bytes.extend_from_slice(&recipient_bytes);
		bytes.extend_from_slice(&relayer_bytes);
		bytes.extend_from_slice(&fee_bytes);
		bytes.extend_from_slice(&refund_bytes);
		// TODO: Update gadget being used to include fee as well
		// TODO: This is not currently included in
		// arkworks_gadgets::setup::mixer::get_public_inputs bytes.extend_from_slice(&
		// fee.encode());
		let result = T::Verifier::verify(&bytes, proof_bytes)?;
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);

		<T as pallet::Config<I>>::Currency::transfer(mixer.asset, &Self::account_id(), &recipient, mixer.deposit_size)?;

		Ok(())
	}

	fn add_nullifier_hash(id: T::TreeId, nullifier_hash: T::Element) -> Result<(), DispatchError> {
		NullifierHashes::<T, I>::insert(id, nullifier_hash, true);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> MixerInspector<T::AccountId, CurrencyIdOf<T, I>, T::TreeId, T::Element>
	for Pallet<T, I>
{
	fn get_root(tree_id: T::TreeId) -> Result<T::Element, DispatchError> {
		T::Tree::get_root(tree_id)
	}

	fn is_known_root(tree_id: T::TreeId, target_root: T::Element) -> Result<bool, DispatchError> {
		T::Tree::is_known_root(tree_id, target_root)
	}

	fn is_nullifier_used(tree_id: T::TreeId, nullifier_hash: T::Element) -> bool {
		NullifierHashes::<T, I>::contains_key(tree_id, nullifier_hash)
	}

	fn ensure_known_root(id: T::TreeId, target: T::Element) -> Result<(), DispatchError> {
		let is_known: bool = Self::is_known_root(id, target)?;
		ensure!(is_known, Error::<T, I>::InvalidWithdrawRoot);
		Ok(())
	}

	fn ensure_nullifier_unused(id: T::TreeId, nullifier: T::Element) -> Result<(), DispatchError> {
		ensure!(
			!Self::is_nullifier_used(id, nullifier),
			Error::<T, I>::AlreadyRevealedNullifier
		);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn get_mixer(
		id: T::TreeId,
	) -> Result<MixerMetadata<T::AccountId, BalanceOf<T, I>, CurrencyIdOf<T, I>>, DispatchError> {
		let mixer = Self::mixers(id);
		ensure!(mixer.is_some(), Error::<T, I>::NoMixerFound);
		Ok(mixer.unwrap())
	}
}

/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
