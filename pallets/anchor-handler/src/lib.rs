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
//! Add description #TODO
//!
//! ## Overview
//!
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The anchor handler system in Webb is designed to make the following
//! possible:
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

use webb_primitives::{
	anchor::AnchorConfig,
	traits::anchor::{AnchorInspector, AnchorInterface},
	ResourceId,
};
use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::EnsureOrigin};
use frame_system::pallet_prelude::OriginFor;
use sp_std::prelude::*;
use pallet_anchor::{BalanceOf, CurrencyIdOf};
use pallet_linkable_tree::types::EdgeMetadata;
pub mod types;
use types::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use pallet_anchor::AnchorConfigration;
	use pallet_linkable_tree::types::EdgeMetadata;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_anchor::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// Anchor Interface
		type Anchor: AnchorInterface<AnchorConfigration<Self, I>> + AnchorInspector<AnchorConfigration<Self, I>>;
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
		/// This will be called by bridge when proposal to create an
		/// anchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_anchor_create_proposal(
			origin: OriginFor<T>,
			deposit_size: BalanceOf<T, I>,
			src_chain_id: T::ChainId,
			r_id: ResourceId,
			max_edges: u32,
			tree_depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::create_anchor(deposit_size, src_chain_id, r_id, max_edges, tree_depth, asset)
		}

		/// This will be called by bridge when proposal to add/update edge of an
		/// anchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_anchor_update_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			anchor_metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::update_anchor(r_id, anchor_metadata)
		}
	}
}

impl<T: Config<I>, I: 'static> AnchorConfig for Pallet<T, I> {
	type AccountId = T::AccountId;
	type Balance = BalanceOf<T, I>;
	type ChainId = T::ChainId;
	type CurrencyId = CurrencyIdOf<T, I>;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn create_anchor(
		deposit_size: BalanceOf<T, I>,
		src_chain_id: T::ChainId,
		r_id: ResourceId,
		max_edges: u32,
		tree_depth: u8,
		asset: CurrencyIdOf<T, I>,
	) -> DispatchResultWithPostInfo {
		ensure!(
			!AnchorList::<T, I>::contains_key(r_id),
			Error::<T, I>::ResourceIsAlreadyAnchored
		);
		let tree_id = T::Anchor::create(None, deposit_size, tree_depth, max_edges, asset)?;
		AnchorList::<T, I>::insert(r_id, tree_id);
		Counts::<T, I>::insert(src_chain_id, 0);
		Self::deposit_event(Event::AnchorCreated);
		Ok(().into())
	}

	fn update_anchor(
		r_id: ResourceId,
		anchor_metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
	) -> DispatchResultWithPostInfo {
		let tree_id = AnchorList::<T, I>::try_get(r_id).map_err(|_| Error::<T, I>::AnchorHandlerNotFound)?;
		let (src_chain_id, merkle_root, block_height) = (
			anchor_metadata.src_chain_id,
			anchor_metadata.root,
			anchor_metadata.latest_leaf_index,
		);

		if T::Anchor::has_edge(tree_id, src_chain_id) {
			T::Anchor::update_edge(tree_id, src_chain_id, merkle_root, block_height)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::Anchor::add_edge(tree_id, src_chain_id, merkle_root, block_height)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}
		let nonce = Counts::<T, I>::try_get(src_chain_id).map_err(|_| Error::<T, I>::SourceChainIdNotFound)?;
		let record = UpdateRecord {
			tree_id,
			resource_id: r_id,
			edge_metadata: anchor_metadata,
		};
		UpdateRecords::<T, I>::insert(src_chain_id, nonce, record);
		Counts::<T, I>::mutate(src_chain_id, |val| -> DispatchResultWithPostInfo {
			*val = val.checked_add(1).ok_or(Error::<T, I>::StorageOverflow)?;
			Ok(().into())
		})
	}
}
