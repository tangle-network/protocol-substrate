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

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod types;
use codec::{Decode, Encode};
use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	mixer::{MixerInspector, MixerInterface},
	verifier::*,
	ElementTrait,
};
use frame_support::{ensure, pallet_prelude::DispatchError, traits::Get};
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;
use types::*;

pub use pallet::*;

pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

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
		type Mixer: MixerInterface<Self::AccountId, BalanceOf<Self, I>, CurrencyIdOf<Self, I>, Self::TreeId, Self::Element>
			+ MixerInspector<Self::AccountId, CurrencyIdOf<Self, I>, Self::TreeId, Self::Element>;

		/// The verifier
		type Verifier: VerifierModule;

		/// Currency type for taking deposits
		type Currency: MultiCurrency<Self::AccountId>;

		/// The pruning length for neighbor root histories
		type HistoryLength: Get<Self::RootIndex>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchors)]
	pub type Anchors<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, AnchorMetadata<T::AccountId, BalanceOf<T, I>>, ValueQuery>;

	/// The map of trees to the maximum number of anchor edges they can have
	#[pallet::storage]
	#[pallet::getter(fn max_edges)]
	pub type MaxEdges<T: Config<I>, I: 'static = ()> = StorageMap<_, Blake2_128Concat, T::TreeId, u32, ValueQuery>;

	/// The map of trees and chain ids to their edge metadata
	#[pallet::storage]
	#[pallet::getter(fn edge_list)]
	pub type EdgeList<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::ChainId,
		EdgeMetadata<T::ChainId, T::Element, T::BlockNumber>,
		ValueQuery,
	>;

	/// A helper map for denoting whether an anchor is bridged to given chain
	#[pallet::storage]
	#[pallet::getter(fn anchor_has_edge)]
	pub type AnchorHasEdge<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), bool, ValueQuery>;

	/// The map of (tree, chain id) pairs to their latest recorded merkle root
	#[pallet::storage]
	#[pallet::getter(fn neighbor_roots)]
	pub type NeighborRoots<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), Blake2_128Concat, T::RootIndex, T::Element>;

	/// The next neighbor root index to store the merkle root update record
	#[pallet::storage]
	#[pallet::getter(fn curr_neighbor_root_index)]
	pub type CurrentNeighborRootIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), T::RootIndex, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
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
		TooManyEdges,
		/// Edge already exists
		EdgeAlreadyExists,
		EdgeDoesntExists,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			deposit_size: BalanceOf<T, I>,
			max_edges: u32,
			depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id = <Self as AnchorInterface<_, _, _, _, _, _, _>>::create(
				T::AccountId::default(),
				deposit_size,
				depth,
				max_edges,
				asset,
			)?;
			Self::deposit_event(Event::AnchorCreation(tree_id));
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn deposit(origin: OriginFor<T>, tree_id: T::TreeId, leaf: T::Element) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as AnchorInterface<_, _, _, _, _, _, _>>::deposit(origin, tree_id, leaf)?;
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

impl<T: Config<I>, I: 'static>
	AnchorInterface<
		T::BlockNumber,
		T::AccountId,
		BalanceOf<T, I>,
		CurrencyIdOf<T, I>,
		T::ChainId,
		T::TreeId,
		T::Element,
	> for Pallet<T, I>
{
	fn create(
		creator: T::AccountId,
		deposit_size: BalanceOf<T, I>,
		depth: u8,
		max_edges: u32,
		asset: CurrencyIdOf<T, I>,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::Mixer::create(creator, deposit_size, depth, asset.into())?;
		MaxEdges::<T, I>::insert(id, max_edges);
		Ok(id)
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
		_fee: BalanceOf<T, I>,
		_refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		// Check if local root is known
		T::Mixer::ensure_known_root(id, roots[0])?;
		if roots.len() > 1 {
			for i in 1..roots.len() {
				<Self as AnchorInspector<_, _, _, _, _>>::ensure_known_neighbor_root(
					id,
					T::ChainId::from(i as u32),
					roots[i],
				)?;
			}
		}

		// Check nullifier and add or return `InvalidNullifier`
		T::Mixer::ensure_nullifier_unused(id, nullifier_hash)?;
		T::Mixer::add_nullifier_hash(id, nullifier_hash)?;
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
		height: T::BlockNumber,
	) -> Result<(), DispatchError> {
		// ensure edge doesn't exists
		ensure!(
			!EdgeList::<T, I>::contains_key(id, src_chain_id),
			Error::<T, I>::EdgeAlreadyExists
		);
		// ensure anchor isn't at maximum edges
		let max_edges: u32 = Self::max_edges(id);
		let curr_length = EdgeList::<T, I>::iter_prefix_values(id).into_iter().count();
		ensure!(max_edges > curr_length as u32, Error::<T, I>::TooManyEdges);
		// craft edge
		let e_meta = EdgeMetadata::<T::ChainId, T::Element, T::BlockNumber> {
			src_chain_id,
			root,
			height,
		};
		// update historical neighbor list for this edge's root
		let neighbor_root_inx = CurrentNeighborRootIndex::<T, I>::get((id, src_chain_id));
		CurrentNeighborRootIndex::<T, I>::insert(
			(id, src_chain_id),
			neighbor_root_inx + T::RootIndex::one() % T::HistoryLength::get(),
		);
		NeighborRoots::<T, I>::insert((id, src_chain_id), neighbor_root_inx, root);
		// Append new edge to the end of the edge list for the given tree
		EdgeList::<T, I>::insert(id, src_chain_id, e_meta);
		Ok(())
	}

	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		height: T::BlockNumber,
	) -> Result<(), DispatchError> {
		ensure!(
			EdgeList::<T, I>::contains_key(id, src_chain_id),
			Error::<T, I>::EdgeDoesntExists
		);
		let e_meta = EdgeMetadata::<T::ChainId, T::Element, T::BlockNumber> {
			src_chain_id,
			root,
			height,
		};
		let neighbor_root_inx =
			CurrentNeighborRootIndex::<T, I>::get((id, src_chain_id)) + T::RootIndex::one() % T::HistoryLength::get();
		CurrentNeighborRootIndex::<T, I>::insert((id, src_chain_id), neighbor_root_inx);
		NeighborRoots::<T, I>::insert((id, src_chain_id), neighbor_root_inx, root);
		EdgeList::<T, I>::insert(id, src_chain_id, e_meta);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> AnchorInspector<T::AccountId, CurrencyIdOf<T, I>, T::ChainId, T::TreeId, T::Element>
	for Pallet<T, I>
{
	fn get_neighbor_roots(tree_id: T::TreeId) -> Result<Vec<T::Element>, DispatchError> {
		let edges = EdgeList::<T, I>::iter_prefix_values(tree_id)
			.into_iter()
			.collect::<Vec<EdgeMetadata<_, _, _>>>();
		let roots = edges.iter().map(|e| e.root).collect::<Vec<_>>();
		Ok(roots)
	}

	fn is_known_neighbor_root(
		tree_id: T::TreeId,
		src_chain_id: T::ChainId,
		target_root: T::Element,
	) -> Result<bool, DispatchError> {
		if target_root.is_zero() {
			return Ok(false);
		}

		let get_next_inx = |inx: T::RootIndex| {
			if inx.is_zero() {
				T::HistoryLength::get().saturating_sub(One::one())
			} else {
				inx.saturating_sub(One::one())
			}
		};

		let curr_root_inx = CurrentNeighborRootIndex::<T, I>::get((tree_id, src_chain_id));
		let mut historical_root = NeighborRoots::<T, I>::get((tree_id, src_chain_id), curr_root_inx)
			.unwrap_or(T::Element::from_bytes(&[0; 32]));
		if target_root == historical_root {
			return Ok(true);
		}

		let mut i = get_next_inx(curr_root_inx);

		while i != curr_root_inx {
			historical_root =
				NeighborRoots::<T, I>::get((tree_id, src_chain_id), i).unwrap_or(T::Element::from_bytes(&[0; 32]));
			if target_root == historical_root {
				return Ok(true);
			}

			if i == Zero::zero() {
				i = T::HistoryLength::get();
			}

			i -= One::one();
		}

		Ok(false)
	}

	fn has_edge(id: T::TreeId, src_chain_id: T::ChainId) -> bool {
		EdgeList::<T, I>::contains_key(id, src_chain_id)
	}

	fn ensure_known_neighbor_root(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		target: T::Element,
	) -> Result<(), DispatchError> {
		let is_known = Self::is_known_neighbor_root(id, src_chain_id, target)?;
		ensure!(is_known, Error::<T, I>::InvalidNeighborWithdrawRoot);
		Ok(())
	}
}
