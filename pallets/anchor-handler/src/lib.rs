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

use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	traits::{Currency, EnsureOrigin},
};
use frame_system::pallet_prelude::OriginFor;
pub use pallet::*;
use pallet_anchor::types::{AnchorInspector, AnchorInterface, EdgeMetadata};
use sp_std::prelude::*;

use pallet_bridge::types::ResourceId;
pub mod types;
use types::*;

// ChainId is available in both bridge and anchor pallet
type ChainId<T> = <T as pallet_anchor::Config>::ChainId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use pallet_anchor::types::{AnchorInspector, EdgeMetadata};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config + pallet_anchor::Config + pallet_bridge::Config {
		/// The overarching event type.
		type Event: IsType<<Self as frame_system::Config>::Event> + From<Event<Self>>;

		/// Specifies the origin check provided by the bridge for calls that can
		/// only be called by the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// The currency mechanism.
		type Currency: Currency<Self::AccountId>;

		/// Anchor Interface
		type Anchor: AnchorInterface<Self> + AnchorInspector<Self>;
	}

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchors)]
	pub type Anchors<T: Config> = StorageMap<_, Blake2_128Concat, ResourceId, T::TreeId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn update_records)]
	/// sourceChainID => height => Update Record
	pub type UpdateRecords<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChainId<T>,
		Blake2_128Concat,
		T::BlockNumber,
		UpdateRecord<T::AccountId, ResourceId, ChainId<T>, T::Element, T::BlockNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn counts)]
	/// The number of updates
	pub(super) type Counts<T: Config> = StorageMap<_, Blake2_128Concat, ChainId<T>, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", ResourceId = "ResourceId")]
	pub enum Event<T: Config> {
		MaintainerSet(T::AccountId, T::AccountId),
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Access violation.
		InvalidPermissions,
		/// Storage overflowed.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// This will be called by bridge when proposal to create an
		/// anchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_anchor_create_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			max_edges: u32,
			tree_depth: u8,
		) -> DispatchResultWithPostInfo {
			Self::ensure_bridge_origin(origin)?;
			Self::create_anchor(r_id, max_edges, tree_depth)
		}

		/// This will be called by bridge when proposal to add/update edge of an
		/// anchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		pub fn execute_anchor_update_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			anchor_metadata: EdgeMetadata<ChainId<T>, T::Element, T::BlockNumber>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_bridge_origin(origin)?;
			Self::update_anchor(r_id, anchor_metadata)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn ensure_bridge_origin(origin: T::Origin) -> DispatchResultWithPostInfo {
		T::BridgeOrigin::ensure_origin(origin)?;
		Ok(().into())
	}

	fn create_anchor(r_id: ResourceId, max_edges: u32, tree_depth: u8) -> DispatchResultWithPostInfo {
		let tree_id = T::Anchor::create(T::AccountId::default(), max_edges, tree_depth)?;
		Anchors::<T>::insert(r_id, tree_id);
		Self::deposit_event(Event::AnchorCreated);
		Ok(().into())
	}

	fn update_anchor(
		r_id: ResourceId,
		anchor_metadata: EdgeMetadata<ChainId<T>, T::Element, T::BlockNumber>,
	) -> DispatchResultWithPostInfo {
		let tree_id = Anchors::<T>::get(r_id);
		let (src_chain_id, merkle_root, block_height) = (
			anchor_metadata.src_chain_id,
			anchor_metadata.root,
			anchor_metadata.height,
		);

		if T::Anchor::has_edge(tree_id, src_chain_id) {
			T::Anchor::update_edge(tree_id, src_chain_id, merkle_root, block_height)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::Anchor::add_edge(tree_id, src_chain_id, merkle_root, block_height)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}
		let old = Counts::<T>::get(src_chain_id);
		let nonce = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
		let record = UpdateRecord {
			tree_id,
			resource_id: r_id,
			edge_metadata: anchor_metadata,
		};
		// UpdateRecords::<T>::insert(src_chain_id, nonce, record);

		Ok(().into())
	}
}
