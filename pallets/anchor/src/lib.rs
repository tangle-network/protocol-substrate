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

//! # Anchor Module
//!
//! A simple module for building Anchors.
//!
//! ## Overview
//!
//! The Anchor module provides functionality for the following:
//!
//! * Inserting elements to the tree
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The Anchor system in Webb is designed to make the following possible:
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

// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// mod tests;

pub mod types;
use types::*;
use codec::{Decode, Encode, Input};
use frame_support::{ensure, pallet_prelude::DispatchError};
use pallet_mt::types::{ElementTrait};
use pallet_mixer::types::{MixerInspector, MixerInterface, MixerMetadata};

use darkwebb_primitives::verifier::*;
use frame_support::traits::{Currency, ExistenceRequirement::AllowDeath, Get, ReservableCurrency};
use frame_system::Config as SystemConfig;
use sp_runtime::traits::{AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;

pub use pallet::*;

type BalanceOf<T, I = ()> = <<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

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
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_mixer::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// ChainID for anchor edges
		type ChainId: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// The mixer type
		type Mixer: MixerInterface<Self, I> + MixerInspector<Self, I>;

		/// The verifier
		type Verifier: VerifierModule;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their metadata
	#[pallet::storage]
	#[pallet::getter(fn anchors)]
	pub type Anchors<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, AnchorMetadata<T::AccountId, BalanceOf<T, I>>, ValueQuery>;

	/// The map of trees to the maximum number of anchor edges they can have
	#[pallet::storage]
	#[pallet::getter(fn max_edges)]
	pub type MaxEdges<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, u32, ValueQuery>;


	/// The map of trees to their metadata
	#[pallet::storage]
	#[pallet::getter(fn edges)]
	pub type Edges<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, Vec<EdgeMetadata<T::ChainId, T::Element, T::BlockNumber>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn neighbor_roots)]
	pub type NeighborRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(T::TreeId, T::ChainId),
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
	>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_neighbor_root_index)]
	pub type NextNeighborRootIndex<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		(T::TreeId, T::ChainId),
		T::RootIndex,
		ValueQuery
	>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", T::TreeId = "TreeId")]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet(T::AccountId, T::AccountId),
		/// New tree created
		AnchorCreation(T::TreeId),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
		/// Invalid neighbor root passed in withdrawal
		/// (neighbor root is not in neighbor history)
		InvalidNeighborWithdrawRoot,
		/// Anchor is at maximum number of edges for the given tree
		TooManyEdges
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			max_edges: u32,			
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id = T::Mixer::create(T::AccountId::default(), 32u8)?;
			MaxEdges::<T, I>::insert(tree_id, max_edges);

			Self::deposit_event(Event::AnchorCreation(tree_id));
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn deposit(origin: OriginFor<T>, tree_id: T::TreeId, leaf: T::Element) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			T::Mixer::deposit(origin, tree_id, leaf);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
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
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
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

impl<T: Config<I>, I: 'static> AnchorInterface<T, I> for Pallet<T, I> {
	fn create(creator: T::AccountId, depth: u8) -> Result<T::TreeId, DispatchError> {
		T::Mixer::create(T::AccountId::default(), 32u8)
	}

	fn deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), DispatchError> {
		T::Mixer::deposit(depositor, id, leaf)
	}

	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		roots: Vec<T::Element>,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		// Check if local root is known
		T::Mixer::ensure_known_root(id, roots[0])?;
		if roots.len() > 1 {
			for i in 1..roots.len() {
				<Self as AnchorInspector<_,_>>::ensure_known_neighbor_root(
					id,
					T::ChainId::from(i as u32),
					roots[i]
				)?;
			}
		}

		// Check nullifier and add or return `InvalidNullifier`
		T::Mixer::ensure_nullifier_unused(id, nullifier_hash)?;
		T::Mixer::add_nullifier_hash(id, nullifier_hash);
		// Format proof public inputs for verification 
		// FIXME: This is for a specfic gadget so we ought to create a generic handler
		// FIXME: Such as a unpack/pack public inputs trait
		// FIXME: 	-> T::PublicInputTrait::validate(public_bytes: &[u8])
		let mut bytes = vec![];
		bytes.extend_from_slice(&nullifier_hash.encode());
		for i in 0..roots.len() {
			bytes.extend_from_slice(&roots[i].encode());
		}
		bytes.extend_from_slice(&recipient.encode());
		bytes.extend_from_slice(&relayer.encode());
		// TODO: Update gadget being used to include fee as well
		// TODO: This is not currently included in
		// arkworks_gadgets::setup::mixer::get_public_inputs bytes.extend_from_slice(&
		// fee.encode());
		let result = <T as pallet::Config<I>>::Verifier::verify(&bytes, proof_bytes)?;
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);
		// TODO: Transfer assets to the recipient
		Ok(())
	}

	fn add_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		height: T::BlockNumber
	) -> Result<(), DispatchError> {
		let edges: Vec<EdgeMetadata<_,_,_>> = Self::edges(id);
		let max_edges: u32 = Self::max_edges(id);
		ensure!(max_edges > edges.len() as u32, Error::<T, I>::TooManyEdges);
		let e_meta = EdgeMetadata::<T::ChainId, T::Element, T::BlockNumber> {
			src_chain_id: src_chain_id,
			root: root,
			height: height,
		};
		// Append new edge to the end of the edge list for the given tree
		Edges::<T, I>::append(id, e_meta);
		Ok(())
	}

	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		height: T::BlockNumber
	) -> Result<(), DispatchError> {
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> AnchorInspector<T, I> for Pallet<T, I> {
	fn get_neighbor_roots(tree_id: T::TreeId) -> Result<Vec<T::Element>, DispatchError> {
		Ok(vec![T::Element::default()])
	}

	fn is_known_neighbor_root(
		tree_id: T::TreeId,
		src_chain_id: T::ChainId,
		target_root: T::Element
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}