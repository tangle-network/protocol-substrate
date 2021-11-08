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
use frame_support::{
	dispatch::{DispatchResult, DispatchResultWithPostInfo},
	ensure,
	pallet_prelude::*,
};
use frame_system::{pallet_prelude::*, Config as SystemConfig};
use pallet_anchor::{types::EdgeMetadata, AnchorConfigration, PostDepositHook};
use sp_std::prelude::*;
use xcm::latest::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_utils;

pub mod types;
pub use pallet::*;

pub type ChainIdOf<T, I> = <T as pallet_anchor::Config<I>>::ChainId;
pub type ElementOf<T, I> = <T as pallet_mt::Config<I>>::Element;
pub type LeafIndexOf<T, I> = <T as pallet_mt::Config<I>>::LeafIndex;
pub type EdgeMetadataOf<T, I> = EdgeMetadata<ChainIdOf<T, I>, ElementOf<T, I>, LeafIndexOf<T, I>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use darkwebb_primitives::utils;

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
		type ParaId: Get<ParaId>;
		type XcmSender: SendXcm;
		/// Anchor Interface
		type Anchor: AnchorInterface<AnchorConfigration<Self, I>> + AnchorInspector<AnchorConfigration<Self, I>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of linked anchors cross other chains.
	///
	/// * Key1: [T::ChainId] -> Other chain id (ParachainId).
	/// * Key2: [T::TreeId] -> Local Anchor's tree id.
	/// * Value: [T::TreeId] -> Other chain's Anchor's tree id (a la
	/// `RemoteAnchor`).
	#[pallet::storage]
	#[pallet::getter(fn linked_anchors)]
	pub type LinkedAnchors<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::ChainId, Blake2_128Concat, T::TreeId, T::TreeId, ValueQuery>;

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

		/// Registers this [ResourceId] to an anchor which exists on the other
		/// chain.
		///
		/// [ResourceId] is already contains the anchor's tree id defined on
		/// this chain and the chain id of the other chain (which we are try to
		/// link to). we need also to know the other chain's anchor's tree id to
		/// complete the link process which is provided by `target_tree_id`.
		#[pallet::weight(0)]
		pub fn register_resource_id(
			origin: OriginFor<T>,
			r_id: ResourceId,
			target_tree_id: T::TreeId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			Self::register_new_resource_id(r_id, target_tree_id)?;
			Ok(().into())
		}

		/// A Forced version of [Self::register_resource_id] which can be only
		/// called by the Root.
		#[pallet::weight(0)]
		pub fn force_register_resource_id(
			origin: OriginFor<T>,
			r_id: ResourceId,
			target_tree_id: T::TreeId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::register_new_resource_id(r_id, target_tree_id)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update(
			origin: OriginFor<T>,
			r_id: ResourceId,
			metadata: EdgeMetadataOf<T, I>,
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
			Self::update_anchor(tree_id, metadata)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn force_update(
			origin: OriginFor<T>,
			r_id: ResourceId,
			metadata: EdgeMetadataOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let (tree_id, chain_id) = utils::decode_resource_id::<T::TreeId, T::ChainId>(r_id);
			ensure!(metadata.src_chain_id == chain_id, Error::<T, I>::InvalidPermissions);
			ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
			Self::update_anchor(tree_id, metadata)?;
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn register_new_resource_id(r_id: ResourceId, target_tree_id: T::TreeId) -> DispatchResultWithPostInfo {
		// extract the resource id information
		let (tree_id, chain_id) = utils::decode_resource_id::<T::TreeId, T::ChainId>(r_id);
		// and we need to also ensure that the anchor exists
		ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
		// and not already anchored/linked
		ensure!(
			!LinkedAnchors::<T, I>::contains_key(chain_id, tree_id),
			Error::<T, I>::ResourceIsAlreadyAnchored
		);
		// finally, register the resource id
		LinkedAnchors::<T, I>::insert(chain_id, tree_id, target_tree_id);
		// also, add the new edge to the anchor
		Self::update_anchor(tree_id, EdgeMetadata {
			src_chain_id: chain_id,
			..Default::default()
		})?;
		Ok(().into())
	}

	fn update_anchor(tree_id: T::TreeId, metadata: EdgeMetadataOf<T, I>) -> DispatchResultWithPostInfo {
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
		pallet_mt::Trees::<T, I>::contains_key(tree_id)
	}
}

impl<T: Config<I>, I: 'static> PostDepositHook<T, I> for Pallet<T, I> {
	fn post_deposit(_: T::AccountId, my_tree_id: T::TreeId, _: T::Element) -> DispatchResult {
		// we get the current anchor tree
		let tree = pallet_mt::Trees::<T, I>::get(my_tree_id);
		// extract the root
		let root = tree.root;
		// and the latest leaf index
		let latest_leaf_index = tree.leaf_count;
		// get the current parachain id
		let my_para_id = T::ParaId::get();
		// and construct the metadata
		let metadata = EdgeMetadata {
			src_chain_id: para_id_to_chain_id::<T, I>(my_para_id),
			root,
			latest_leaf_index,
		};
		// now we need an iterator for all the edges connected to this anchor
		let edges = pallet_anchor::EdgeList::<T, I>::iter_prefix_values(my_tree_id);
		// for each edge we do the following:
		// 1. get the target tree id on the other chain (using the other chain id, and
		// my tree id)
		// 2. encode the resource id (target tree id + my chain id).
		// 3. get the target parachain id.
		// 4. construct the update call.
		// 5. and finally, dispatch the update call to other parachain.
		for edge in edges {
			// first, we get the target chain tree id
			let other_chain_id = edge.src_chain_id;
			let target_tree_id = LinkedAnchors::<T, I>::get(other_chain_id, my_tree_id);
			let my_chain_id = metadata.src_chain_id;
			// target_tree_id + my_chain_id
			let r_id = utils::encode_resource_id::<T::TreeId, T::ChainId>(target_tree_id, my_chain_id);
			let other_para_id = chain_id_to_para_id::<T, I>(other_chain_id);
			let update_edge = Transact {
				// we should keep using the OriginKind::Native here
				// as that is the only origin type that gives us the information about
				// the sibling parachain (the caller parachain Id).
				origin_type: OriginKind::Native,
				require_weight_at_most: 1_000_000_000,
				call: <T as Config<I>>::Call::from(Call::<T, I>::update {
					metadata: metadata.clone(),
					r_id,
				})
				.encode()
				.into(),
			};
			let dest = (Parent, Parachain(other_para_id.into()));
			let result = T::XcmSender::send_xcm(dest, Xcm(vec![update_edge]));
			match result {
				Ok(()) => {
					Self::deposit_event(Event::RemoteAnchorEdgeUpdated {
						para_id: other_para_id,
						resource_id: r_id,
					});
				}
				Err(e) => {
					Self::deposit_event(Event::RemoteAnchorEdgeUpdateFailed {
						para_id: other_para_id,
						resource_id: r_id,
						error: e,
					});
				}
			}
		}
		Ok(())
	}
}

#[inline(always)]
pub fn chain_id_to_para_id<T: Config<I>, I: 'static>(chain_id: T::ChainId) -> ParaId {
	let mut chain_id_bytes = [0u8; 4];
	chain_id_bytes.copy_from_slice(&chain_id.encode()[..4]);
	ParaId::from(u32::from_le_bytes(chain_id_bytes))
}

#[inline(always)]
pub fn para_id_to_chain_id<T: Config<I>, I: 'static>(para_id: ParaId) -> T::ChainId {
	T::ChainId::from(u32::from(para_id))
}
