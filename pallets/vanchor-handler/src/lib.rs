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

//! # Anchor Handler Module
//!
//! A module for executing the creation and modification of anchors.
//!
//! ## Overview
//!
//! The anchor-handler module provides functionality for the following:
//!
//! * The creation of anchors from proposals
//! * Updating existing anchors from proposals
//!
//! ## Interface
//!
//! ### Permissioned Functions
//!
//! * `execute_vanchor_create_proposal`: Creates a vanchor from successfully voted on proposal. This
//!   method requires the `origin` to be [T::BridgeOrigin].
//! * `execute_vanchor_update_proposal`: Adds/Updates a vanchor from successfully voted on proposal.
//!   This method requires the `origin` to be [T::BridgeOrigin].
//!
//! ## Related Modules
//!
//! * VAnchor pallet
//! * Linkable-tree pallet

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock_bridge;
#[cfg(test)]
mod tests_bridge;

#[cfg(test)]
pub mod mock_signature_bridge;
#[cfg(test)]
mod tests_signature_bridge;

use core::convert::TryInto;
use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::EnsureOrigin};
use frame_system::pallet_prelude::OriginFor;
use pallet_linkable_tree::types::EdgeMetadata;
use pallet_vanchor::{BalanceOf as VAnchorBalanceOf, CurrencyIdOf as VAnchorCurrencyIdOf};
use webb_primitives::{
	traits::vanchor::{VAnchorConfig, VAnchorInspector, VAnchorInterface},
	ResourceId,
};
pub mod types;
use types::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use pallet_linkable_tree::types::EdgeMetadata;
	use pallet_vanchor::VAnchorConfigration;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_vanchor::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// VAnchor Interface
		type VAnchor: VAnchorInterface<VAnchorConfigration<Self, I>>
			+ VAnchorInspector<VAnchorConfigration<Self, I>>;
	}

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchor_list)]
	pub type AnchorList<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::TreeId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn update_records)]
	/// sourceChainID => nonce => Update Record
	pub type UpdateRecords<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ChainId,
		Blake2_128Concat,
		u64,
		UpdateRecord<T::TreeId, ResourceId, T::ChainId, T::Element, T::LeafIndex>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn counts)]
	/// The number of updates
	pub(super) type Counts<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::ChainId, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Access violation.
		InvalidPermissions,
		// Anchor handler already exists for specified resource Id.
		ResourceIsAlreadyAnchored,
		// Anchor handler doesn't exist for specified resoure Id.
		AnchorHandlerNotFound,
		// Source chain Id is not registered.
		SourceChainIdNotFound,
		/// Storage overflowed.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// This will be called by bridge when proposal to create a
		/// vanchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_vanchor_create_proposal(
			origin: OriginFor<T>,
			src_chain_id: T::ChainId,
			r_id: ResourceId,
			max_edges: u32,
			tree_depth: u8,
			asset: VAnchorCurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::create_vanchor(src_chain_id, r_id, max_edges, tree_depth, asset)
		}

		/// This will be called by bridge when proposal to add/update edge of a
		/// vanchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_vanchor_update_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			vanchor_metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::update_vanchor(r_id, vanchor_metadata)
		}

		// TODO: Add configurable limit proposal executors for VAnchors
	}
}

impl<T: Config<I>, I: 'static> VAnchorConfig for Pallet<T, I> {
	type AccountId = T::AccountId;
	type Balance = VAnchorBalanceOf<T, I>;
	type Amount = i128;
	type ChainId = T::ChainId;
	type CurrencyId = VAnchorCurrencyIdOf<T, I>;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn create_vanchor(
		src_chain_id: T::ChainId,
		r_id: ResourceId,
		max_edges: u32,
		tree_depth: u8,
		asset: VAnchorCurrencyIdOf<T, I>,
	) -> DispatchResultWithPostInfo {
		ensure!(!AnchorList::<T, I>::contains_key(r_id), Error::<T, I>::ResourceIsAlreadyAnchored);
		let tree_id = T::VAnchor::create(None, tree_depth, max_edges, asset)?;
		AnchorList::<T, I>::insert(r_id, tree_id);
		Counts::<T, I>::insert(src_chain_id, 0);
		Self::deposit_event(Event::AnchorCreated);
		Ok(().into())
	}

	fn update_vanchor(
		r_id: ResourceId,
		anchor_metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
	) -> DispatchResultWithPostInfo {
		let tree_id =
			AnchorList::<T, I>::try_get(r_id).map_err(|_| Error::<T, I>::AnchorHandlerNotFound)?;
		let (src_chain_id, merkle_root, latest_leaf_index, target) = (
			anchor_metadata.src_chain_id,
			anchor_metadata.root,
			anchor_metadata.latest_leaf_index,
			anchor_metadata.target,
		);

		if T::VAnchor::has_edge(tree_id, src_chain_id) {
			T::VAnchor::update_edge(tree_id, src_chain_id, merkle_root, latest_leaf_index, target)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::VAnchor::add_edge(tree_id, src_chain_id, merkle_root, latest_leaf_index, target)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}
		let nonce = Counts::<T, I>::try_get(src_chain_id)
			.map_err(|_| Error::<T, I>::SourceChainIdNotFound)?;
		let record = UpdateRecord { tree_id, resource_id: r_id, edge_metadata: anchor_metadata };
		UpdateRecords::<T, I>::insert(src_chain_id, nonce, record);
		Counts::<T, I>::mutate(src_chain_id, |val| -> DispatchResultWithPostInfo {
			*val = val.checked_add(1).ok_or(Error::<T, I>::StorageOverflow)?;
			Ok(().into())
		})
	}
}
