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

use codec::Encode;
use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
use cumulus_primitives_core::ParaId;
use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	utils, ResourceId,
};
use frame_support::dispatch::{DispatchResult, DispatchResultWithPostInfo};
use frame_system::Config as SystemConfig;
use pallet_anchor::{types::EdgeMetadata, AnchorConfigration, PostDepositHook};
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
	use darkwebb_primitives::utils;
	use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_anchor::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<<Self as SystemConfig>::Origin> + Into<Result<CumulusOrigin, <Self as Config<I>>::Origin>>;

		/// The overarching call type; we assume sibling chains use the same
		/// type.
		type Call: From<Call<Self, I>> + Encode;

		type XcmSender: SendXcm;
		/// Anchor Interface
		type Anchor: AnchorInterface<AnchorConfigration<Self, I>> + AnchorInspector<AnchorConfigration<Self, I>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchor_list)]
	pub type AnchorList<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::TreeId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[allow(clippy::large_enum_variant)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet {
			old_maintainer: T::AccountId,
			new_maintainer: T::AccountId,
		},
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
		RemoteAnchorEdgeUpdated {
			para_id: ParaId,
			resource_id: ResourceId,
		},
		RemoteAnchorEdgeUpdateFailed {
			para_id: ParaId,
			resource_id: ResourceId,
			error: SendError,
		},
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

		#[pallet::weight(0)]
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

		#[pallet::weight(0)]
		pub fn register_resource_id(origin: OriginFor<T>, r_id: ResourceId) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// also ensure that this resource id is not already anchored
			ensure!(
				!AnchorList::<T, I>::contains_key(r_id),
				Error::<T, I>::ResourceIsAlreadyAnchored
			);
			// extract the resource id information
			let (tree_id, _) = utils::decode_resource_id::<T::TreeId, T::ChainId>(r_id);
			// and finally, ensure that the anchor exists
			ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
			// register the resource id
			AnchorList::<T, I>::insert(r_id, tree_id);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn force_register_resource_id(origin: OriginFor<T>, r_id: ResourceId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			// ensure that this resource id is not already anchored
			ensure!(
				!AnchorList::<T, I>::contains_key(r_id),
				Error::<T, I>::ResourceIsAlreadyAnchored
			);
			// extract the resource id information
			let (tree_id, _) = utils::decode_resource_id::<T::TreeId, T::ChainId>(r_id);
			// and finally, ensure that the anchor exists
			ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
			// register the resource id
			AnchorList::<T, I>::insert(r_id, tree_id);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update(
			origin: OriginFor<T>,
			r_id: ResourceId,
			metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
		) -> DispatchResultWithPostInfo {
			let para = ensure_sibling_para(<T as Config<I>>::Origin::from(origin))?;
			let caller_chain_id = T::ChainId::from(u32::from(para));
			let (tree_id, r_chain_id) = utils::decode_resource_id::<T::TreeId, T::ChainId>(r_id);
			// double check that the caller is the same as the chain id of the resource
			// also the the same from the metadata.
			ensure!(
				caller_chain_id == metadata.src_chain_id && caller_chain_id == r_chain_id,
				Error::<T, I>::InvalidPermissions
			);
			// and finally, ensure that the anchor exists
			ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
			// now we can update the anchor
			Self::update_anchor(r_id, metadata)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn force_update(
			origin: OriginFor<T>,
			r_id: ResourceId,
			metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::update_anchor(r_id, metadata)?;
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn update_anchor(
		r_id: ResourceId,
		metadata: EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
	) -> DispatchResultWithPostInfo {
		let tree_id = AnchorList::<T, I>::try_get(r_id).map_err(|_| Error::<T, I>::AnchorHandlerNotFound)?;
		if T::Anchor::has_edge(tree_id, metadata.src_chain_id) {
			T::Anchor::update_edge(
				tree_id,
				metadata.src_chain_id,
				metadata.root,
				metadata.latest_leaf_index,
			)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::Anchor::add_edge(
				tree_id,
				metadata.src_chain_id,
				metadata.root,
				metadata.latest_leaf_index,
			)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}

		Ok(().into())
	}

	fn anchor_exists(tree_id: T::TreeId) -> bool {
		pallet_mixer::Mixers::<T, I>::get(tree_id).is_some()
	}
}

impl<T: Config<I>, I: 'static> PostDepositHook<T, I> for Pallet<T, I> {
	fn post_deposit(_: T::AccountId, id: T::TreeId, _: T::Element) -> DispatchResult {
		// we get the current anchor tree
		let tree = pallet_mt::Trees::<T, I>::get(id);
		// extract the root
		let root = tree.root;
		// and the latest leaf index
		let latest_leaf_index = tree.leaf_count;
		// get the current parachain id
		// FIXME: get the chain id somehow from some pallet
		let para = ParaId::from(2000u32);
		// and construct the metadata
		let metadata = EdgeMetadata {
			src_chain_id: T::ChainId::from(u32::from(para)),
			root,
			latest_leaf_index,
		};
		// now we need an iterator for all the edges connected to this anchor
		let edges = pallet_anchor::EdgeList::<T, I>::iter_prefix_values(id);
		// for each edge we do the following:
		// 1. encode the resource id (my tree id + target chain id).
		// 2. get the target parachain id.
		// 3. construct the update call.
		// 4. and finally, dispatch the update call to other parachain.
		for edge in edges {
			let r_id = utils::encode_resource_id::<T::TreeId, T::ChainId>(id, edge.src_chain_id);
			let chain_id_bytes = edge.src_chain_id.encode();
			let mut chain_id = [0u8; 4];
			chain_id.copy_from_slice(&chain_id_bytes[..4]);
			let chain_id = u32::from_le_bytes(chain_id);
			let para_id = ParaId::from(chain_id);
			let update_edge = Transact {
				origin_type: OriginKind::Native,
				require_weight_at_most: 1_000,
				call: <T as Config<I>>::Call::from(Call::<T, I>::update {
					metadata: metadata.clone(),
					r_id,
				})
				.encode()
				.into(),
			};
			let dest = (1, Junction::Parachain(para_id.into()));
			let result = T::XcmSender::send_xcm(dest, Xcm(vec![update_edge]));
			match result {
				Ok(()) => {
					Self::deposit_event(Event::RemoteAnchorEdgeUpdated {
						para_id,
						resource_id: r_id,
					});
				}
				Err(e) => {
					Self::deposit_event(Event::RemoteAnchorEdgeUpdateFailed {
						para_id,
						resource_id: r_id,
						error: e,
					});
				}
			}
		}
		Ok(())
	}
}
