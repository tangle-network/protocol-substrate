// This file is part of Webb.

// Copyright (C) 2021-2023 Webb Technologies Inc.
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
pub mod mock_signature_bridge;
#[cfg(test)]
mod tests_signature_bridge;

use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::EnsureOrigin};
use frame_system::pallet_prelude::OriginFor;
use pallet_vanchor::{BalanceOf as VAnchorBalanceOf, CurrencyIdOf as VAnchorCurrencyIdOf};
use sp_std::convert::TryInto;
use webb_primitives::{
	traits::vanchor::{VAnchorConfig, VAnchorInspector, VAnchorInterface},
	webb_proposals::{ResourceId, TargetSystem},
};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use pallet_vanchor::VAnchorConfiguration;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]

	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_vanchor::Config<I> {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type BridgeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// VAnchor Interface
		type VAnchor: VAnchorInterface<VAnchorConfiguration<Self, I>>
			+ VAnchorInspector<VAnchorConfiguration<Self, I>>;
	}

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchor_list)]
	pub type AnchorList<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::TreeId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
		ResourceAnchored,
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
		/// Invalid nonce
		InvalidNonce,
		/// Invalid resource ID
		InvalidResourceId,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// This will be called by bridge when proposal to create a
		/// vanchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(0)]
		pub fn execute_vanchor_create_proposal(
			origin: OriginFor<T>,
			src_chain_id: T::ChainId,
			r_id: ResourceId,
			max_edges: u32,
			tree_depth: u8,
			asset: VAnchorCurrencyIdOf<T, I>,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::create_vanchor(src_chain_id, r_id, max_edges, tree_depth, asset, nonce)
		}

		/// This will be called by bridge when proposal to add/update edge of a
		/// vanchor has been successfully voted on.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(1)]
		pub fn execute_vanchor_update_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			merkle_root: T::Element,
			src_resource_id: ResourceId,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			let tree_id: T::TreeId = match r_id.target_system() {
				TargetSystem::Substrate(system) => system.tree_id.into(),
				_ => {
					ensure!(false, Error::<T, I>::InvalidResourceId);
					T::TreeId::default()
				},
			};
			Self::update_vanchor(tree_id, merkle_root, src_resource_id, nonce.into())
		}

		/// This will by called by bridge when proposal to set new resource for
		/// handler has been successfully voted on.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(2)]
		pub fn execute_set_resource_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			let tree_id: T::TreeId = match r_id.target_system() {
				TargetSystem::Substrate(system) => system.tree_id.into(),
				_ => 0u32.into(),
			};
			Self::set_resource(r_id, tree_id)
		}

		/// Execute set max deposit limit proposal.
		/// The `MaxDepositLimitProposal` updates the maximum deposit amount allowed on the variable
		/// anchor system.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(3)]
		pub fn execute_set_max_deposit_limit_proposal(
			origin: OriginFor<T>,
			max_deposit_limit: VAnchorBalanceOf<T, I>,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;

			Self::set_max_deposit_amount(max_deposit_limit, nonce)
		}

		/// Execute set min withdrawal limit proposal.
		/// The `MinWithdrawalLimitProposal` updates the minimum withdrawal amount allowed on the
		/// variable anchor system.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(4)]
		pub fn execute_set_min_withdrawal_limit_proposal(
			origin: OriginFor<T>,
			min_withdraw_limit: VAnchorBalanceOf<T, I>,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;

			Self::set_min_withdraw_amount(min_withdraw_limit, nonce)
		}
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
	type ProposalNonce = T::ProposalNonce;
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn set_resource(r_id: ResourceId, tree_id: T::TreeId) -> DispatchResultWithPostInfo {
		ensure!(!AnchorList::<T, I>::contains_key(r_id), Error::<T, I>::ResourceIsAlreadyAnchored);
		AnchorList::<T, I>::insert(r_id, tree_id);
		Self::deposit_event(Event::ResourceAnchored);
		Ok(().into())
	}

	fn create_vanchor(
		_src_chain_id: T::ChainId,
		r_id: ResourceId,
		max_edges: u32,
		tree_depth: u8,
		asset: VAnchorCurrencyIdOf<T, I>,
		nonce: T::ProposalNonce,
	) -> DispatchResultWithPostInfo {
		ensure!(!AnchorList::<T, I>::contains_key(r_id), Error::<T, I>::ResourceIsAlreadyAnchored);
		let tree_id = T::VAnchor::create(None, tree_depth, max_edges, asset, nonce)?;
		_ = Self::set_resource(r_id, tree_id);
		Self::deposit_event(Event::AnchorCreated);
		Ok(().into())
	}

	fn update_vanchor(
		tree_id: T::TreeId,
		merkle_root: T::Element,
		src_resource_id: ResourceId,
		latest_leaf_index: T::LeafIndex,
	) -> DispatchResultWithPostInfo {
		let src_chain_id = src_resource_id.typed_chain_id().chain_id().into();
		if T::VAnchor::has_edge(tree_id, src_chain_id) {
			T::VAnchor::update_edge(
				tree_id,
				src_chain_id,
				merkle_root,
				latest_leaf_index,
				src_resource_id,
			)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::VAnchor::add_edge(
				tree_id,
				src_chain_id,
				merkle_root,
				latest_leaf_index,
				src_resource_id,
			)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}
		Ok(().into())
	}

	fn set_max_deposit_amount(
		max_deposit_limit: VAnchorBalanceOf<T, I>,
		nonce: T::ProposalNonce,
	) -> DispatchResultWithPostInfo {
		T::VAnchor::set_max_deposit_amount(max_deposit_limit, nonce)?;
		Self::deposit_event(Event::AnchorEdgeAdded);
		Ok(().into())
	}

	fn set_min_withdraw_amount(
		min_withdraw_limit: VAnchorBalanceOf<T, I>,
		nonce: T::ProposalNonce,
	) -> DispatchResultWithPostInfo {
		T::VAnchor::set_min_withdraw_amount(min_withdraw_limit, nonce)?;
		Self::deposit_event(Event::AnchorEdgeAdded);
		Ok(().into())
	}
}
