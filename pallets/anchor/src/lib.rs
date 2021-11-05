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
mod test_utils;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod zk_config;

mod benchmarking;

pub mod types;
pub mod weights;
use codec::{Decode, Encode};
use darkwebb_primitives::{
	anchor::{AnchorConfig, AnchorInspector, AnchorInterface},
	merkle_tree::{TreeInspector, TreeInterface},
	verifier::*,
	ElementTrait,
};
use frame_support::{dispatch::DispatchResult, ensure, pallet_prelude::DispatchError, traits::Get};
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AccountIdConversion, AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;
use types::*;
pub use weights::WeightInfo;

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> =
	<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_mt::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// ChainID for anchor edges
		type ChainId: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// The tree type
		type LinkableTree: TreeInterface<Self::AccountId, Self::TreeId, Self::Element>
			+ TreeInspector<Self::AccountId, Self::TreeId, Self::Element>;

		/// The verifier
		type Verifier: VerifierModule;

		/// Currency type for taking deposits
		type Currency: MultiCurrency<Self::AccountId>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		/// The pruning length for neighbor root histories
		type HistoryLength: Get<Self::RootIndex>;

		/// A type that implements [PostDepositHook] that would be called when a
		/// deposit is made.
		type PostDepositHook: PostDepositHook<Self, I>;

		/// Weight info for pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchors)]
	pub type Anchors<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Option<AnchorMetadata<T::AccountId, BalanceOf<T, I>, CurrencyIdOf<T, I>>>,
		ValueQuery,
	>;

	/// The map of trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn nullifier_hashes)]
	pub type NullifierHashes<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::TreeId, Blake2_128Concat, T::Element, bool, ValueQuery>;

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
		EdgeMetadata<T::ChainId, T::Element, T::LeafIndex>,
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
		MaintainerSet {
			old_maintainer: T::AccountId,
			new_maintainer: T::AccountId,
		},
		/// New tree created
		AnchorCreation { tree_id: T::TreeId },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid Merkle Roots
		InvalidMerkleRoots,
		/// Unknown root
		UnknownRoot,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
		/// Mixer not found.
		NoAnchorFound,
		/// Invalid neighbor root passed in withdrawal
		/// (neighbor root is not in neighbor history)
		InvalidNeighborWithdrawRoot,
		/// Anchor is at maximum number of edges for the given tree
		TooManyEdges,
		/// Edge already exists
		EdgeAlreadyExists,
		/// Edge does not exist
		EdgeDoesntExists,
		/// Invalid nullifier that is already used
		/// (this error is returned when a nullifier is used twice)
		AlreadyRevealedNullifier,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(<T as Config<I>>::WeightInfo::create(*depth as u32, *max_edges))]
		pub fn create(
			origin: OriginFor<T>,
			deposit_size: BalanceOf<T, I>,
			max_edges: u32,
			depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			// Should it only be the root who can create anchors?
			ensure_root(origin)?;
			let tree_id =
				<Self as AnchorInterface<_>>::create(T::AccountId::default(), deposit_size, depth, max_edges, asset)?;
			Self::deposit_event(Event::AnchorCreation { tree_id });
			Ok(().into())
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::deposit())]
		pub fn deposit(origin: OriginFor<T>, tree_id: T::TreeId, leaf: T::Element) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as AnchorInterface<_>>::deposit(origin.clone(), tree_id, leaf)?;
			T::PostDepositHook::post_deposit(origin, tree_id, leaf)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config<I>>::WeightInfo::set_maintainer())]
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

		#[pallet::weight(<T as Config<I>>::WeightInfo::force_set_maintainer())]
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

		#[pallet::weight(<T as Config<I>>::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			id: T::TreeId,
			proof_bytes: Vec<u8>,
			chain_id: T::ChainId,
			roots: Vec<T::Element>,
			nullifier_hash: T::Element,
			recipient: T::AccountId,
			relayer: T::AccountId,
			fee: BalanceOf<T, I>,
			refund: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			<Self as AnchorInterface<_>>::withdraw(
				id,
				proof_bytes.as_slice(),
				chain_id,
				roots,
				nullifier_hash,
				recipient,
				relayer,
				fee,
				refund,
			)?;
			Ok(().into())
		}
	}
}

pub struct AnchorConfigration<T: Config<I>, I: 'static>(core::marker::PhantomData<T>, core::marker::PhantomData<I>);

impl<T: Config<I>, I: 'static> AnchorConfig for AnchorConfigration<T, I> {
	type AccountId = T::AccountId;
	type Balance = BalanceOf<T, I>;
	type ChainId = T::ChainId;
	type CurrencyId = CurrencyIdOf<T, I>;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> AnchorInterface<AnchorConfigration<T, I>> for Pallet<T, I> {
	fn create(
		creator: T::AccountId,
		deposit_size: BalanceOf<T, I>,
		depth: u8,
		max_edges: u32,
		asset: CurrencyIdOf<T, I>,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::LinkableTree::create(creator.clone(), depth)?;
		Anchors::<T, I>::insert(
			id,
			Some(AnchorMetadata {
				creator,
				deposit_size,
				asset,
			}),
		);
		MaxEdges::<T, I>::insert(id, max_edges);
		Ok(id)
	}

	fn deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), DispatchError> {
		// insert the leaf
		T::LinkableTree::insert_in_order(id, leaf)?;

		let anchor = Self::get_anchor(id)?;
		// transfer tokens to the pallet
		<T as Config<I>>::Currency::transfer(anchor.asset, &depositor, &Self::account_id(), anchor.deposit_size)?;

		Ok(())
	}

	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		chain_id: T::ChainId,
		roots: Vec<T::Element>,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		let m = MaxEdges::<T, I>::get(id) as usize;
		// double check the number of roots
		ensure!(roots.len() == m, Error::<T, I>::InvalidMerkleRoots);
		let anchor = Self::get_anchor(id)?;
		// Check if local root is known
		Self::ensure_known_root(id, roots[0])?;
		// Check if neighbor roots are known
		if roots.len() > 1 {
			// Get edges and corresponding chain IDs for the anchor
			let edges = EdgeList::<T, I>::iter_prefix(id).into_iter().collect::<Vec<_>>();

			// Check membership of provided historical neighbor roots
			for (i, (chain_id, _)) in edges.iter().enumerate() {
				<Self as AnchorInspector<_>>::ensure_known_neighbor_root(id, *chain_id, roots[i + 1])?;
			}
		}

		// Check nullifier and add or return `InvalidNullifier`
		Self::ensure_nullifier_unused(id, nullifier_hash)?;
		Self::add_nullifier_hash(id, nullifier_hash)?;
		// Format proof public inputs for verification
		// FIXME: This is for a specfic gadget so we ought to create a generic handler
		// FIXME: Such as a unpack/pack public inputs trait
		// FIXME: 	-> T::PublicInputTrait::validate(public_bytes: &[u8])
		//
		// nullifier_hash (0..32)
		// recipient (32..64)
		// relayer (64..96)
		// fee (96..128)
		// refund (128..160)
		// chain_id (160..192)
		// root[1] (224..256)
		// root[2] (256..288)
		// root[m - 1] (...)
		let mut bytes = vec![];

		let element_encoder = |v: &[u8]| {
			let mut output = [0u8; 32];
			output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
			output
		};
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);
		let fee_bytes = fee.using_encoded(element_encoder);
		let refund_bytes = refund.using_encoded(element_encoder);
		let chain_id_bytes = chain_id.using_encoded(element_encoder);

		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&recipient_bytes);
		bytes.extend_from_slice(&relayer_bytes);
		bytes.extend_from_slice(&fee_bytes);
		bytes.extend_from_slice(&refund_bytes);
		bytes.extend_from_slice(&chain_id_bytes);
		for root in roots.iter().take(m) {
			bytes.extend_from_slice(&root.encode());
		}
		let result = <T as pallet::Config<I>>::Verifier::verify(&bytes, proof_bytes)?;
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);
		// transfer the assets
		<T as Config<I>>::Currency::transfer(anchor.asset, &Self::account_id(), &recipient, anchor.deposit_size)?;
		Ok(())
	}

	fn add_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
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
		let e_meta = EdgeMetadata {
			src_chain_id,
			root,
			latest_leaf_index,
		};
		// update historical neighbor list for this edge's root
		let neighbor_root_idx = CurrentNeighborRootIndex::<T, I>::get((id, src_chain_id));
		CurrentNeighborRootIndex::<T, I>::insert(
			(id, src_chain_id),
			neighbor_root_idx + T::RootIndex::one() % T::HistoryLength::get(),
		);
		NeighborRoots::<T, I>::insert((id, src_chain_id), neighbor_root_idx, root);
		// Append new edge to the end of the edge list for the given tree
		EdgeList::<T, I>::insert(id, src_chain_id, e_meta);
		Ok(())
	}

	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
	) -> Result<(), DispatchError> {
		ensure!(
			EdgeList::<T, I>::contains_key(id, src_chain_id),
			Error::<T, I>::EdgeDoesntExists
		);
		let e_meta = EdgeMetadata {
			src_chain_id,
			root,
			latest_leaf_index,
		};
		let neighbor_root_idx =
			(CurrentNeighborRootIndex::<T, I>::get((id, src_chain_id)) + T::RootIndex::one()) % T::HistoryLength::get();
		CurrentNeighborRootIndex::<T, I>::insert((id, src_chain_id), neighbor_root_idx);
		NeighborRoots::<T, I>::insert((id, src_chain_id), neighbor_root_idx, root);
		EdgeList::<T, I>::insert(id, src_chain_id, e_meta);
		Ok(())
	}

	fn add_nullifier_hash(id: T::TreeId, nullifier_hash: T::Element) -> Result<(), DispatchError> {
		NullifierHashes::<T, I>::insert(id, nullifier_hash, true);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> AnchorInspector<AnchorConfigration<T, I>> for Pallet<T, I> {
	fn get_root(tree_id: T::TreeId) -> Result<T::Element, DispatchError> {
		T::LinkableTree::get_root(tree_id)
	}

	fn is_known_root(tree_id: T::TreeId, target_root: T::Element) -> Result<bool, DispatchError> {
		T::LinkableTree::is_known_root(tree_id, target_root)
	}

	fn ensure_known_root(id: T::TreeId, target: T::Element) -> Result<(), DispatchError> {
		let is_known: bool = Self::is_known_root(id, target)?;
		ensure!(is_known, Error::<T, I>::UnknownRoot);
		Ok(())
	}

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
			.unwrap_or_else(|| T::Element::from_bytes(&[0; 32]));
		if target_root == historical_root {
			return Ok(true);
		}

		let mut i = get_next_inx(curr_root_inx);

		while i != curr_root_inx {
			historical_root = NeighborRoots::<T, I>::get((tree_id, src_chain_id), i)
				.unwrap_or_else(|| T::Element::from_bytes(&[0; 32]));
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

	fn is_nullifier_used(tree_id: T::TreeId, nullifier_hash: T::Element) -> bool {
		NullifierHashes::<T, I>::contains_key(tree_id, nullifier_hash)
	}

	fn ensure_nullifier_unused(id: T::TreeId, nullifier: T::Element) -> Result<(), DispatchError> {
		ensure!(
			!Self::is_nullifier_used(id, nullifier),
			Error::<T, I>::AlreadyRevealedNullifier
		);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn get_anchor(
		id: T::TreeId,
	) -> Result<AnchorMetadata<T::AccountId, BalanceOf<T, I>, CurrencyIdOf<T, I>>, DispatchError> {
		let anchor = Anchors::<T, I>::get(id);
		ensure!(anchor.is_some(), Error::<T, I>::NoAnchorFound);
		Ok(anchor.unwrap())
	}
}

pub trait PostDepositHook<T: Config<I>, I: 'static> {
	fn post_deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> DispatchResult;
}

impl<T: Config<I>, I: 'static> PostDepositHook<T, I> for () {
	fn post_deposit(_: T::AccountId, _: T::TreeId, _: T::Element) -> DispatchResult {
		Ok(())
	}
}
/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
