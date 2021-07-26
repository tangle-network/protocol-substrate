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

//! # Merkle Tree Module
//!
//! A simple module for building incremental merkle trees.
//!
//! ## Overview
//!
//! The Merkle Tree module provides functionality for SMT operations
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
//! The Merkle Tree system in Webb is designed to make the following possible:
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

pub mod types;
use types::{MixerInterface, MixerMetadata};

use codec::{Encode, Decode, Input};
use frame_support::pallet_prelude::DispatchError;
use frame_support::ensure;
use pallet_mt::types::{TreeInterface, TreeInspector, TreeMetadata, ElementTrait};

use sp_std::{prelude::*};
use sp_runtime::{
	traits::{Saturating, AtLeast32Bit, One, Zero}
};
use frame_support::traits::{Currency, ReservableCurrency, Get, ExistenceRequirement::AllowDeath};
use darkwebb_primitives::{verifier::*};
use frame_system::Config as SystemConfig;

pub use pallet::*;

type BalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo},
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_mt::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The tree
		type Tree: TreeInterface<Self, I> + TreeInspector<Self, I>;

		/// The verifier
		type Verifier: VerifierModule;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		T::AccountId,
		ValueQuery,
	>;

	/// The map of trees to their metadata
	#[pallet::storage]
	#[pallet::getter(fn mixers)]
	pub type Mixers<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		MixerMetadata<T::AccountId, BalanceOf<T, I>>,
		ValueQuery
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		T::TreeId = "TreeId",
	)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet(T::AccountId, T::AccountId),
		/// New tree created
		MixerCreation(T::TreeId),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id = T::Tree::create(T::AccountId::default(), 32u8)?;

			Self::deposit_event(Event::MixerCreation(tree_id));
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn deposit(
			origin: OriginFor<T>,
			tree_id: T::TreeId,
			leaf: T::Element,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as MixerInterface<_,_>>::deposit(origin, tree_id, leaf);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(origin, new_maintainer));
				Ok(().into())
			})

		}

		#[pallet::weight(0)]
		pub fn force_set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(Default::default(), T::AccountId::default()));
				Ok(().into())
			})

		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn two() -> T::LeafIndex {
		let two: T::LeafIndex = {
			let one: T::LeafIndex = One::one();
			one.saturating_add(One::one())
		};

		two
	}
}

impl<T: Config<I>, I: 'static> MixerInterface<T, I> for Pallet<T, I> {
	fn deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), DispatchError> {
		// insert the leaf
		T::Tree::insert_in_order(
			id,
			leaf,
		)?;

		let mixer = Self::mixers(id);
		// transfer tokens to the pallet
		<T as pallet::Config<I>>::Currency::transfer(&depositor, &T::AccountId::default(), mixer.deposit_size, AllowDeath)?;

		Ok(())
	}

	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		let root = T::Tree::get_root(id)?;
		let mut bytes = vec![];
		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&root.encode());
		bytes.extend_from_slice(&recipient.encode());
		bytes.extend_from_slice(&relayer.encode());
		// TODO: Update gadget being used to include fee as well
		// TODO: This is not currently included in arkworks_gadgets::setup::mixer::get_public_inputs
		// bytes.extend_from_slice(&fee.encode());
		let result = <T as pallet::Config<I>>::Verifier::verify(&bytes, proof_bytes)?;
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);
		Ok(())
	}
}
