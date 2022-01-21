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
use frame_support::{
	dispatch::{DispatchResult, DispatchResultWithPostInfo},
	ensure,
	pallet_prelude::*,
};
use frame_system::{pallet_prelude::*, Config as SystemConfig};
use pallet_anchor::{AnchorConfigration, PostDepositHook};
use pallet_linkable_tree::types::EdgeMetadata;
use sp_std::prelude::*;
use webb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	utils, ResourceId,
};
use xcm::latest::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;
pub use pallet::*;
use types::*;

pub type ChainIdOf<T, I> = <T as pallet_linkable_tree::Config<I>>::ChainId;
pub type ElementOf<T, I> = <T as pallet_mt::Config<I>>::Element;
pub type TreeIdOf<T, I> = <T as pallet_mt::Config<I>>::TreeId;
pub type LeafIndexOf<T, I> = <T as pallet_mt::Config<I>>::LeafIndex;
pub type EdgeMetadataOf<T, I> = EdgeMetadata<ChainIdOf<T, I>, ElementOf<T, I>, LeafIndexOf<T, I>>;
pub type LinkProposalOf<T, I> = LinkProposal<ChainIdOf<T, I>, TreeIdOf<T, I>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use pallet_anchor::BalanceOf;
	use webb_primitives::utils;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_anchor::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config<I>>::Origin>>;

		/// The overarching call type; we assume sibling chains use the same
		/// type.
		type Call: From<Call<Self, I>> + Encode;
		type ParaId: Get<ParaId>;
		type XcmSender: SendXcm;
		type DemocracyGovernanceDelegate: DemocracyGovernanceDelegate<
			Self,
			<Self as Config<I>>::Call,
			BalanceOf<Self, I>,
		>;
		type DemocracyOrigin: EnsureOrigin<<Self as SystemConfig>::Origin>;
		/// Anchor Interface
		type Anchor: AnchorInterface<AnchorConfigration<Self, I>>
			+ AnchorInspector<AnchorConfigration<Self, I>>;
	}
	/// The map of *eventually* linked anchors cross other chains.
	///
	/// * Key1: [T::ChainId] -> Other chain id (ParachainId).
	/// * Key2: [T::TreeId] -> Local Anchor's tree id.
	/// * Value: [Option<T::TreeId>] -> Other chain's Anchor's tree id (a la
	/// `RemoteAnchor`) or empty, in case if we don't know yet the target tree
	/// id.
	#[pallet::storage]
	#[pallet::getter(fn pending_linked_anchors)]
	pub type PendingLinkedAnchors<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ChainId,
		Blake2_128Concat,
		T::TreeId,
		Option<T::TreeId>,
		ValueQuery,
	>;

	/// The map of linked anchors cross other chains.
	///
	/// * Key1: [T::ChainId] -> Other chain id (ParachainId).
	/// * Key2: [T::TreeId] -> Local Anchor's tree id.
	/// * Value: [T::TreeId] -> Other chain's Anchor's tree id (a la
	/// `RemoteAnchor`).
	#[pallet::storage]
	#[pallet::getter(fn linked_anchors)]
	pub type LinkedAnchors<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ChainId,
		Blake2_128Concat,
		T::TreeId,
		T::TreeId,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[allow(clippy::large_enum_variant)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet { old_maintainer: T::AccountId, new_maintainer: T::AccountId },
		AnchorCreated,
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
		RemoteAnchorEdgeUpdated { para_id: ParaId, resource_id: ResourceId },
		RemoteAnchorEdgeUpdateFailed { para_id: ParaId, resource_id: ResourceId, error: SendError },
		SendingLinkProposalFailed { para_id: ParaId, error: SendError },
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
		/// The Link Process for this anchor is already running.
		AnchorLinkIsAlreadyPending,
		/// Sending Link Proposal To the other chain failed.
		SendingLinkProposalFailed,
		/// Anchor Link is not found!
		AnchorLinkNotFound,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Creates a new Proposal to link two anchors cross-chain
		/// by creating on-chain proposal that once passed will send to the
		/// other chain a link proposal.
		#[pallet::weight(0)]
		pub fn propose_to_link_anchor(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
			value: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin.clone())?;
			// first we check if the anchor exists locally
			ensure!(Self::anchor_exists(payload.local_tree_id), Error::<T, I>::AnchorNotFound);
			// then we do check if it is not linked to the other chain already.
			ensure!(
				!LinkedAnchors::<T, I>::contains_key(
					payload.target_chain_id,
					payload.local_tree_id
				),
				Error::<T, I>::ResourceIsAlreadyAnchored
			);
			// we double check again, if there is not pending link for this.
			ensure!(
				!PendingLinkedAnchors::<T, I>::contains_key(
					payload.target_chain_id,
					payload.local_tree_id
				),
				Error::<T, I>::AnchorLinkIsAlreadyPending
			);
			// add the proposal to the pending link storage.
			PendingLinkedAnchors::<T, I>::insert(
				payload.target_chain_id,
				payload.local_tree_id,
				payload.target_tree_id,
			);
			let proposal = <T as Config<I>>::Call::from(Call::<T, I>::send_link_anchor_message {
				payload,
				value,
			});
			// finally we can create the proposal.
			T::DemocracyGovernanceDelegate::propose(origin, proposal, value)?;
			Ok(().into())
		}

		/// Once a proposal is passed, this function will send a Link proposal
		/// to the other chain also, save the proposal hash locally so when the
		/// other chain passes the proposal, we get signled back with the
		/// proposal hash and we link the anchors.
		///
		/// **Note**: This method requires the `origin` to be
		/// [T::DemocracyOrigin].
		#[pallet::weight(0)]
		pub fn send_link_anchor_message(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
			value: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_democracy(origin)?;
			// this means that the proposal for link anchors on this chain is passed.
			// we need now to signal the other chain to start a link proposal on that chain
			// too.
			let other_para_id = chain_id_to_para_id::<T, I>(payload.target_chain_id);
			let my_para_id = T::ParaId::get();
			let payload = LinkProposal {
				target_chain_id: para_id_to_chain_id::<T, I>(my_para_id),
				..payload
			};
			let save_link_proposal = Transact {
				origin_type: OriginKind::Native,
				require_weight_at_most: 1_000_000_000,
				call: <T as Config<I>>::Call::from(Call::<T, I>::save_link_proposal {
					payload: payload.clone(),
				})
				.encode()
				.into(),
			};
			let handle_link_anchor_message = Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: 1_000_000_000,
				call: <T as Config<I>>::Call::from(Call::<T, I>::handle_link_anchor_message {
					payload,
					value,
				})
				.encode()
				.into(),
			};
			let dest = (Parent, Parachain(other_para_id.into()));
			T::XcmSender::send_xcm(dest, Xcm(vec![save_link_proposal, handle_link_anchor_message]))
				.map_err(|_| Error::<T, I>::SendingLinkProposalFailed)?;
			Ok(().into())
		}

		/// **Note**: This method requires the `origin` to be a sibling
		/// parachain.
		#[pallet::weight(0)]
		pub fn save_link_proposal(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let para = ensure_sibling_para(<T as Config<I>>::Origin::from(origin))?;
			let caller_chain_id = para_id_to_chain_id::<T, I>(para);
			// now we on the other chain (if you look at it from the caller point of view)
			// here, we do first check if the requested anchor exists (if any).
			let my_tree_id = match payload.target_tree_id {
				Some(tree_id) => {
					ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
					tree_id
				},
				None => todo!("create an anchor if the caller does not provide one"),
			};
			// next, we check if the anchor is not linked to the local chain already.
			ensure!(
				!LinkedAnchors::<T, I>::contains_key(caller_chain_id, my_tree_id),
				Error::<T, I>::ResourceIsAlreadyAnchored
			);
			// we double check again, if there is not a pending link for this
			// anchor/proposal.
			ensure!(
				!PendingLinkedAnchors::<T, I>::contains_key(caller_chain_id, my_tree_id),
				Error::<T, I>::AnchorLinkIsAlreadyPending
			);
			// now we save the link proposal.
			PendingLinkedAnchors::<T, I>::insert(
				caller_chain_id,
				my_tree_id,
				Some(payload.local_tree_id),
			);
			Ok(().into())
		}

		/// Handles the Link anchor proposal from other chain, by creating an
		/// on-chain proposal that once passed will link the anchors on the
		/// local chain, also signals back the caller chain with the proposal
		/// hash, so the caller chain know that the link process is complete.
		#[pallet::weight(0)]
		pub fn handle_link_anchor_message(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
			value: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let _caller = ensure_signed(origin.clone())?;
			let my_tree_id = match payload.target_tree_id {
				Some(tree_id) => {
					ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
					tree_id
				},
				None => todo!("create an anchor if the caller does not provide one"),
			};
			// double check that it is already in the pending link storage.
			ensure!(
				PendingLinkedAnchors::<T, I>::contains_key(payload.target_chain_id, my_tree_id),
				Error::<T, I>::AnchorLinkNotFound
			);
			// finally, we create the on-chain proposal, that's when passed will link the
			// anchors locally and send back to the other chain that the link process is
			// done.
			let payload = LinkProposal {
				target_tree_id: Some(payload.local_tree_id),
				local_tree_id: my_tree_id,
				..payload
			};
			let proposal = <T as Config<I>>::Call::from(Call::<T, I>::link_anchors { payload });
			// finally we can create the proposal.
			T::DemocracyGovernanceDelegate::propose(origin, proposal, value)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn link_anchors(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_democracy(origin)?;
			// at this point both chains agress to link the anchors.
			// so we link them locally, and also signal back to the other chain that
			// requested the link that the link process is done.
			let other_para_id = chain_id_to_para_id::<T, I>(payload.target_chain_id);
			ensure!(
				PendingLinkedAnchors::<T, I>::contains_key(
					payload.target_chain_id,
					payload.local_tree_id
				),
				Error::<T, I>::AnchorLinkNotFound,
			);
			// now we can remove the pending linked anchor.
			PendingLinkedAnchors::<T, I>::remove(payload.target_chain_id, payload.local_tree_id);
			let r_id = utils::encode_resource_id(payload.local_tree_id, payload.target_chain_id);
			// unwrap here is safe, since we are sure that it has the value of the tree id.
			let target_tree_id = payload.target_tree_id.unwrap();
			// it is now ready to link them locally.
			Self::register_new_resource_id(r_id, target_tree_id)?;
			// next, we signal back to the other chain that the link process is done.
			let handle_link_anchor = Transact {
				origin_type: OriginKind::Native,
				require_weight_at_most: 1_000_000_000,
				call: <T as Config<I>>::Call::from(Call::<T, I>::handle_link_anchors { payload })
					.encode()
					.into(),
			};
			let dest = (Parent, Parachain(other_para_id.into()));
			T::XcmSender::send_xcm(dest, Xcm(vec![handle_link_anchor]))
				.map_err(|_| Error::<T, I>::SendingLinkProposalFailed)?;
			Ok(().into())
		}

		/// Handles the signal back from the other parachain, if the link
		/// process is there is done to complete the link process here too.
		#[pallet::weight(0)]
		pub fn handle_link_anchors(
			origin: OriginFor<T>,
			payload: LinkProposalOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let para = ensure_sibling_para(<T as Config<I>>::Origin::from(origin))?;
			let caller_chain_id = para_id_to_chain_id::<T, I>(para);
			// get the local tree id, it should be in the target_tree_id.
			let my_tree_id = match payload.target_tree_id {
				Some(tree_id) => {
					ensure!(Self::anchor_exists(tree_id), Error::<T, I>::AnchorNotFound);
					tree_id
				},
				None => return Err(Error::<T, I>::AnchorNotFound.into()),
			};
			// if we are here, on this chain, that means this chain it is the one who
			// started the link process means, that we should find the anchor in the linked
			// anchors list.
			ensure!(
				PendingLinkedAnchors::<T, I>::contains_key(caller_chain_id, my_tree_id),
				Error::<T, I>::AnchorLinkNotFound,
			);
			// now we can remove the pending linked anchor.
			PendingLinkedAnchors::<T, I>::remove(caller_chain_id, my_tree_id);
			let r_id = utils::encode_resource_id(my_tree_id, caller_chain_id);
			let target_tree_id = payload.local_tree_id;
			// it is now ready to link them locally.
			Self::register_new_resource_id(r_id, target_tree_id)?;
			// Link process is done!, Yay!
			Ok(().into())
		}

		/// Registers this [ResourceId] to an anchor which exists on the other
		/// chain.
		///
		/// [ResourceId] is already contains the anchor's tree id defined on
		/// this chain and the chain id of the other chain (which we are try to
		/// link to). we need also to know the other chain's anchor's tree id to
		/// complete the link process which is provided by `target_tree_id`.
		///
		/// **Note**: Only could be called by [T::DemocracyOrigin].
		#[pallet::weight(0)]
		pub fn register_resource_id(
			origin: OriginFor<T>,
			r_id: ResourceId,
			target_tree_id: T::TreeId,
		) -> DispatchResultWithPostInfo {
			Self::ensure_democracy(origin)?;
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

		/// Sync All the Anchors in this chain to the other chains that are
		/// already linked.
		#[pallet::weight(0)]
		pub fn sync_anchors(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			let anchors = pallet_anchor::Anchors::<T, I>::iter_keys();
			for anchor in anchors {
				Self::sync_anchor(anchor)?;
			}
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn register_new_resource_id(
		r_id: ResourceId,
		target_tree_id: T::TreeId,
	) -> DispatchResultWithPostInfo {
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
		Self::update_anchor(
			tree_id,
			EdgeMetadata { src_chain_id: chain_id, ..Default::default() },
		)?;
		Ok(().into())
	}

	fn update_anchor(
		tree_id: T::TreeId,
		metadata: EdgeMetadataOf<T, I>,
	) -> DispatchResultWithPostInfo {
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

	fn ensure_democracy(o: OriginFor<T>) -> DispatchResultWithPostInfo {
		T::DemocracyOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(().into())
	}

	/// Sync Anchor Edge with other parachains that linked to that anchor
	/// usinc XCM.
	fn sync_anchor(tree_id: T::TreeId) -> DispatchResult {
		// we get the current anchor tree
		let tree = match pallet_mt::Trees::<T, I>::get(tree_id) {
			Some(t) => t,
			None => return Err(Error::<T, I>::AnchorNotFound.into()),
		};
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
		let edges = pallet_linkable_tree::EdgeList::<T, I>::iter_prefix_values(tree_id);
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
			let target_tree_id = LinkedAnchors::<T, I>::get(other_chain_id, tree_id);
			let my_chain_id = metadata.src_chain_id;
			// target_tree_id + my_chain_id
			let r_id =
				utils::encode_resource_id::<T::TreeId, T::ChainId>(target_tree_id, my_chain_id);
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
				},
				Err(e) => {
					Self::deposit_event(Event::RemoteAnchorEdgeUpdateFailed {
						para_id: other_para_id,
						resource_id: r_id,
						error: e,
					});
				},
			}
		}

		Ok(())
	}
}

impl<T: Config<I>, I: 'static> PostDepositHook<T, I> for Pallet<T, I> {
	fn post_deposit(_: T::AccountId, my_tree_id: T::TreeId, _: T::Element) -> DispatchResult {
		Self::sync_anchor(my_tree_id)
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
