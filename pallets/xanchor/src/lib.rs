// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License")
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

//! # xAnchor Module
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
//! The xAnchor system in Webb is designed to make the following
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

#![cfg_attr(not(feature = "std"), no_std)]

use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
use cumulus_primitives_core::ParaId;
use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	ResourceId,
};
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_system::Config as SystemConfig;
use pallet_anchor::{types::EdgeMetadata, AnchorConfigration};
use sp_std::prelude::*;
use xcm::latest::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_anchor::Config<I>
	{
		/// The overarching event type.
		type Event: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config<I>>::Origin>>;

		/// The overarching call type; we assume sibling chains use the same
		/// type.
		type Call: From<Call<Self, I>> + Encode;

		type XcmSender: SendXcm;
		/// Anchor Interface
		type Anchor: AnchorInterface<AnchorConfigration<Self, I>>
			+ AnchorInspector<AnchorConfigration<Self, I>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchor_list)]
	pub type AnchorList<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::TreeId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet {
			old_maintainer: T::AccountId,
			new_maintainer: T::AccountId,
		},
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// The anchor is not found
		AnchorNotFound,
		/// Anchor handler doesn't exist for specified resoure Id.
		AnchorHandlerNotFound,
		/// Anchor handler already exists for specified resource Id.
		ResourceIsAlreadyAnchored,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(
				origin == Self::maintainer(),
				Error::<T, I>::InvalidPermissions
			);
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

		#[pallet::weight(0)]
		pub fn force_set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
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

		#[pallet::weight(0)]
		pub fn register_resource_id_for_anchor(
			origin: OriginFor<T>,
			r_id: ResourceId,
			anchor_id: T::TreeId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(
				origin == Self::maintainer(),
				Error::<T, I>::InvalidPermissions
			);
			// also ensure that this resource id is not already anchored
			ensure!(
				!AnchorList::<T, I>::contains_key(r_id),
				Error::<T, I>::ResourceIsAlreadyAnchored
			);
			// and finally, ensure that the anchor exists
			ensure!(
				Self::anchor_exists(anchor_id),
				Error::<T, I>::AnchorNotFound
			);
			// register the resource id
			AnchorList::<T, I>::insert(r_id, anchor_id);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn link(
			origin: OriginFor<T>,
			r_id: ResourceId,
			metadata: EdgeMetadata<T::ChainId, T::Element, T::BlockNumber>,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(
				origin == Self::maintainer(),
				Error::<T, I>::InvalidPermissions
			);
			// link that parachain anchor to the local anchor
			Self::update_anchor(r_id, metadata)?;
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn update_anchor(
		r_id: ResourceId,
		metadata: EdgeMetadata<T::ChainId, T::Element, T::BlockNumber>,
	) -> DispatchResultWithPostInfo {
		let tree_id = AnchorList::<T, I>::try_get(r_id)
			.map_err(|_| Error::<T, I>::AnchorHandlerNotFound)?;
		if T::Anchor::has_edge(tree_id, metadata.src_chain_id) {
			T::Anchor::update_edge(
				tree_id,
				metadata.src_chain_id,
				metadata.root,
				metadata.height,
			)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::Anchor::add_edge(
				tree_id,
				metadata.src_chain_id,
				metadata.root,
				metadata.height,
			)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}

		Ok(().into())
	}

	fn anchor_exists(tree_id: T::TreeId) -> bool {
		pallet_mixer::Mixers::<T, I>::get(tree_id).is_some()
	}
}
