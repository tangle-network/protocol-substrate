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

//! # Linkable-tree Module
//!
//! A module for constructing, modifying and inspecting linkable trees.
//!
//! ## Overview
//!
//! The Linkable-tree module provides functionality for the following:
//!
//! * Creating new linkable trees
//! * Inserting new leafs to a specified tree
//! * Adding an edge to a specified tree
//! * Updating an edge to a specified tree
//! * Inspecting a tree's state
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! * **EdgeList**: A map of trees and chain ids to their edge metadata.
//!  
//! ### Goals
//!
//! The Linkable-tree in Webb is designed to make the following possible:
//!
//! * Store edges of neighboring anchors’ merkle roots
//! * Store historoical data about neighboring merkle roots
//!
//! ## LinkableTreeInterface Interface
//!
//! `create`: Creates a new linkable tree.
//! `insert_in_order`: Inserts new leaf to the tree specified by provided id.
//! `add_edge`: Adds an edge to tree specified by provided id.
//! `update_edge`: Updates an edge to tree specified by provided id.
//!
//! ## LinkableTreeInspector Interface
//!
//! `get_chain_id`: Creates a new linkable tree.
//! `is_known_root`: Checks if a merkle root is in a tree's cached history or returns.
//! `ensure_known_root`: Ensure that passed root is in history.
//! `get_root`: Gets the merkle root for a tree or returns `TreeDoesntExist`.
//! `get_neighbor_roots`: Gets the merkle root for a tree or returns `TreeDoesntExist`.
//! `is_known_neighbor_root`: Checks if a merkle root is in a tree's cached history or returns
//! `TreeDoesntExist`. `ensure_known_neighbor_roots`: Checks if each root from passed root array is
//! in tree's cached history or returns `InvalidNeighborWithdrawRoot`. `ensure_known_neighbor_root`:
//! Checks if a merkle root is in a tree's cached history or returns `InvalidNeighborWithdrawRoot`.
//! `has_edge`: Check if this linked tree has this edge.
//! `ensure_max_edges`: Check if passed number of roots is the same as max allowed edges or returns
//! `InvalidMerkleRoots`.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod benchmarking;

pub mod types;
pub mod weights;
use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::DispatchError, traits::Get};
use sp_runtime::traits::{AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;
use types::*;
use webb_primitives::{
	traits::{linkable_tree::*, merkle_tree::*},
	utils::compute_chain_id_type,
	ElementTrait,
};
pub use weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_mt::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// ChainID for anchor edges
		type ChainId: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// ChainID type for this chain
		#[pallet::constant]
		type ChainType: Get<[u8; 2]>;

		// Getter of id of the current chain
		#[pallet::constant]
		type ChainIdentifier: Get<Self::ChainId>;

		/// The tree
		type Tree: TreeInterface<Self::AccountId, Self::TreeId, Self::Element>
			+ TreeInspector<Self::AccountId, Self::TreeId, Self::Element>;

		/// The pruning length for neighbor root histories
		#[pallet::constant]
		type HistoryLength: Get<Self::RootIndex>;

		/// Weight info for pallet
		type WeightInfo: WeightInfo;
	}

	/// The map of trees to the maximum number of anchor edges they can have
	#[pallet::storage]
	#[pallet::getter(fn max_edges)]
	pub type MaxEdges<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, u32, ValueQuery>;

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

	/// A helper map for denoting whether an tree is bridged to given chain
	#[pallet::storage]
	#[pallet::getter(fn linkable_tree_has_edge)]
	pub type LinkableTreeHasEdge<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), bool, ValueQuery>;

	/// The map of (tree, chain id) pairs to their latest recorded merkle root
	#[pallet::storage]
	#[pallet::getter(fn neighbor_roots)]
	pub type NeighborRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(T::TreeId, T::ChainId),
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
	>;

	/// The next neighbor root index to store the merkle root update record
	#[pallet::storage]
	#[pallet::getter(fn curr_neighbor_root_index)]
	pub type CurrentNeighborRootIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), T::RootIndex, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New tree created
		LinkableTreeCreation { tree_id: T::TreeId },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		// Root is not found in history
		UnknownRoot,
		/// Invalid Merkle Roots
		InvalidMerkleRoots,
		/// Invalid neighbor root passed in withdrawal
		/// (neighbor root is not in neighbor history)
		InvalidNeighborWithdrawRoot,
		/// Anchor is at maximum number of edges for the given tree
		TooManyEdges,
		/// Edge already exists
		EdgeAlreadyExists,
		/// Edge does not exist
		EdgeDoesntExists,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(<T as Config<I>>::WeightInfo::create(*depth as u32, *max_edges))]
		pub fn create(
			origin: OriginFor<T>,
			max_edges: u32,
			depth: u8,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id = <Self as LinkableTreeInterface<_>>::create(None, max_edges, depth)?;
			Self::deposit_event(Event::LinkableTreeCreation { tree_id });
			Ok(().into())
		}
	}
}

pub struct LinkableTreeConfigration<T: Config<I>, I: 'static>(
	core::marker::PhantomData<T>,
	core::marker::PhantomData<I>,
);

impl<T: Config<I>, I: 'static> LinkableTreeConfig for LinkableTreeConfigration<T, I> {
	type AccountId = T::AccountId;
	type ChainId = T::ChainId;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> LinkableTreeInterface<LinkableTreeConfigration<T, I>>
	for Pallet<T, I>
{
	fn create(
		creator: Option<T::AccountId>,
		max_edges: u32,
		depth: u8,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::Tree::create(creator, depth)?;
		MaxEdges::<T, I>::insert(id, max_edges);
		Ok(id)
	}

	fn insert_in_order(id: T::TreeId, leaf: T::Element) -> Result<T::Element, DispatchError> {
		T::Tree::insert_in_order(id, leaf)
	}

	fn add_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
		target: T::Element,
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
		let e_meta = EdgeMetadata { src_chain_id, root, latest_leaf_index, target };
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
		latest_leaf_index: T::LeafIndex,
		target: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(EdgeList::<T, I>::contains_key(id, src_chain_id), Error::<T, I>::EdgeDoesntExists);
		let e_meta = EdgeMetadata { src_chain_id, root, latest_leaf_index, target };
		let neighbor_root_inx = (CurrentNeighborRootIndex::<T, I>::get((id, src_chain_id)) +
			T::RootIndex::one()) %
			T::HistoryLength::get();
		CurrentNeighborRootIndex::<T, I>::insert((id, src_chain_id), neighbor_root_inx);
		NeighborRoots::<T, I>::insert((id, src_chain_id), neighbor_root_inx, root);
		EdgeList::<T, I>::insert(id, src_chain_id, e_meta);
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> LinkableTreeInspector<LinkableTreeConfigration<T, I>>
	for Pallet<T, I>
{
	fn get_chain_id() -> T::ChainId {
		T::ChainIdentifier::get()
	}

	fn get_chain_id_type() -> T::ChainId {
		T::ChainId::try_from(compute_chain_id_type(T::ChainIdentifier::get(), T::ChainType::get()))
			.unwrap_or_default()
	}

	fn get_chain_type() -> [u8; 2] {
		T::ChainType::get()
	}

	fn get_root(id: T::TreeId) -> Result<T::Element, DispatchError> {
		T::Tree::get_root(id)
	}

	fn is_known_root(id: T::TreeId, root: T::Element) -> Result<bool, DispatchError> {
		T::Tree::is_known_root(id, root)
	}

	fn ensure_known_root(id: T::TreeId, root: T::Element) -> Result<(), DispatchError> {
		let known_root = Self::is_known_root(id, root)?;
		ensure!(known_root, Error::<T, I>::UnknownRoot);
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
			return Ok(false)
		}

		let get_next_inx = |inx: T::RootIndex| {
			if inx.is_zero() {
				T::HistoryLength::get().saturating_sub(One::one())
			} else {
				inx.saturating_sub(One::one())
			}
		};

		let curr_root_inx = CurrentNeighborRootIndex::<T, I>::get((tree_id, src_chain_id));
		let mut historical_root =
			NeighborRoots::<T, I>::get((tree_id, src_chain_id), curr_root_inx)
				.unwrap_or_else(|| T::Element::from_bytes(&[0; 32]));
		if target_root == historical_root {
			return Ok(true)
		}

		let mut i = get_next_inx(curr_root_inx);

		while i != curr_root_inx {
			historical_root = NeighborRoots::<T, I>::get((tree_id, src_chain_id), i)
				.unwrap_or_else(|| T::Element::from_bytes(&[0; 32]));
			if target_root == historical_root {
				return Ok(true)
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

	fn ensure_max_edges(id: T::TreeId, num_roots: usize) -> Result<(), DispatchError> {
		let m = MaxEdges::<T, I>::get(id) as usize;
		ensure!(num_roots == m, Error::<T, I>::InvalidMerkleRoots);
		Ok(())
	}

	fn ensure_known_neighbor_roots(
		id: T::TreeId,
		roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		if roots.len() > 1 {
			// Get edges and corresponding chain IDs for the anchor
			let edges = EdgeList::<T, I>::iter_prefix(id).into_iter().collect::<Vec<_>>();

			// Check membership of provided historical neighbor roots
			for (i, (chain_id, _)) in edges.iter().enumerate() {
				Self::ensure_known_neighbor_root(id, *chain_id, roots[i + 1])?;
			}
		}
		Ok(())
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
