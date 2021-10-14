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

//! # Merkle Tree Module
//!
//! A simple module for building incremental merkle trees.
//!
//! ## Overview
//!
//! The Merkle Tree module provides functionality for SMT operations
//! including:
//!
//! * Inserting elements to the tree
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The Merkle Tree system in Webb is designed to make the following possible:
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

mod benchmarking;

pub mod weights;

pub mod types;
use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::DispatchError};
use types::TreeMetadata;

pub use weights::WeightInfo;

use darkwebb_primitives::{
	hasher::*,
	traits::merkle_tree::{TreeInspector, TreeInterface},
	types::{DepositDetails, ElementTrait},
};
use frame_support::traits::{Currency, Get, ReservableCurrency};
use frame_system::Config as SystemConfig;
use sp_runtime::traits::{AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;

type DepositBalanceOf<T, I = ()> = <<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching tree ID type
		type TreeId: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// The overarching leaf index type
		type LeafIndex: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// The overarching leaf index type
		type RootIndex: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// the leaf type
		type Element: ElementTrait;

		/// the default zero element
		type DefaultZeroElement: Get<Self::Element>;

		/// The max depth of trees
		type MaxTreeDepth: Get<u8>;

		/// The max length of default hashes
		type MaxDefaultHashes: Get<u16>;

		/// The hasher instance trait
		type Hasher: HasherModule;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The origin which may forcibly modify the tree
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		/// The basic amount of funds that must be reserved for an tree.
		type TreeDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved when adding metadata
		/// to your parameters.
		type DataDepositBase: Get<DepositBalanceOf<Self, I>>;

		/// The additional funds that must be reserved for the number of bytes
		/// you store in your parameter metadata.
		type DataDepositPerByte: Get<DepositBalanceOf<Self, I>>;

		/// The value of two in this form
		type Two: Get<DepositBalanceOf<Self, I>>;

		/// History size of roots for each tree
		type RootHistorySize: Get<Self::RootIndex>;

		/// The maximum length of a name or symbol stored on-chain.
		type StringLimit: Get<u32>;

		/// WeightInfo for pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn existing_deposit)]
	/// Details of the module's parameters
	pub(super) type Deposit<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_tree_id)]
	pub(super) type NextTreeId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::TreeId, ValueQuery>;

	/// The map of trees to their metadata
	#[pallet::storage]
	#[pallet::getter(fn trees)]
	pub type Trees<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, TreeMetadata<T::AccountId, T::LeafIndex, T::Element>, ValueQuery>;

	/// The default hashes for this tree pallet
	#[pallet::storage]
	#[pallet::getter(fn default_hashes)]
	pub(super) type DefaultHashes<T: Config<I>, I: 'static = ()> = StorageValue<_, Vec<T::Element>, ValueQuery>;

	/// The map of (tree_id, index) to the leaf commitment
	#[pallet::storage]
	#[pallet::getter(fn leaves)]
	pub(super) type Leaves<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::TreeId, Blake2_128Concat, T::LeafIndex, T::Element, ValueQuery>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_root_index)]
	pub(super) type NextRootIndex<T: Config<I>, I: 'static = ()> = StorageValue<_, T::RootIndex, ValueQuery>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_leaf_index)]
	pub(super) type NextLeafIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, T::LeafIndex, ValueQuery>;

	/// Map of root history from tree id to root index to root values
	#[pallet::storage]
	#[pallet::getter(fn cached_roots)]
	pub(super) type CachedRoots<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::TreeId, Blake2_128Concat, T::RootIndex, T::Element, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		MaintainerSet(T::AccountId, T::AccountId),
		/// New tree created
		TreeCreation(T::TreeId, T::AccountId),
		/// New leaf inserted
		LeafInsertion(T::TreeId, T::LeafIndex, T::Element),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid depth of the tree specified
		InvalidTreeDepth,
		/// Invalid  leaf index,  either taken or too large
		InvalidLeafIndex,
		/// Tree is full
		ExceedsMaxLeaves,
		/// Tree doesnt exist
		TreeDoesntExist,
		/// Invalid length for default hashes
		ExceedsMaxDefaultHashes,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			let mut temp_hashes: Vec<T::Element> = Vec::with_capacity(T::MaxTreeDepth::get() as usize);
			let default_zero = T::DefaultZeroElement::get();
			temp_hashes.push(default_zero);
			let mut temp_hash = default_zero.to_bytes().to_vec();
			for _ in 0..T::MaxTreeDepth::get() {
				temp_hash = T::Hasher::hash_two(&temp_hash, &temp_hash).unwrap();
				temp_hashes.push(T::Element::from_vec(temp_hash.clone()));
			}

			DefaultHashes::<T, I>::put(temp_hashes);
			1u64 + 1u64
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::create(depth as u32))]
		pub fn create(origin: OriginFor<T>, depth: u8) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			ensure!(
				depth <= T::MaxTreeDepth::get() && depth > 0,
				Error::<T, I>::InvalidTreeDepth
			);
			// calculate the deposit, we charge the user based on # of leaves
			let deposit = T::DataDepositPerByte::get()
				.saturating_mul(T::Two::get().saturating_pow(depth.into()))
				.saturating_add(T::DataDepositBase::get());

			T::Currency::reserve(&origin, deposit)?;

			let tree_id = <Self as TreeInterface<_, _, _>>::create(origin.clone(), depth)?;

			Self::deposit_event(Event::TreeCreation(tree_id, origin));
			Ok(().into())
		}

		#[pallet::weight(T::WeightInfo::insert())]
		pub fn insert(origin: OriginFor<T>, tree_id: T::TreeId, leaf: T::Element) -> DispatchResultWithPostInfo {
			let _origin = ensure_signed(origin)?;
			ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
			let tree = Trees::<T, I>::get(tree_id);
			let next_index = Self::next_leaf_index(tree_id);
			ensure!(next_index == tree.leaf_count, Error::<T, I>::InvalidLeafIndex);
			ensure!(
				tree.leaf_count.saturating_add(One::one()) <= tree.max_leaves,
				Error::<T, I>::ExceedsMaxLeaves
			);
			// insert the leaf
			<Self as TreeInterface<_, _, _>>::insert_in_order(tree_id, leaf)?;

			Self::deposit_event(Event::LeafInsertion(tree_id, next_index, leaf));

			Ok(().into())
		}

		#[pallet::weight(T::WeightInfo::set_maintainer())]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(origin, new_maintainer));
				Ok(().into())
			})
		}

		#[pallet::weight(T::WeightInfo::force_set_maintainer())]
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(Default::default(), new_maintainer));
				Ok(().into())
			})
		}

		#[pallet::weight(T::WeightInfo::force_set_default_hashes(default_hashes.len() as u32))]
		pub fn force_set_default_hashes(
			origin: OriginFor<T>,
			default_hashes: Vec<T::Element>,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			let len_of_hashes = default_hashes.len();
			ensure!(
				len_of_hashes > 0 && len_of_hashes <= T::MaxDefaultHashes::get() as usize,
				Error::<T, I>::ExceedsMaxDefaultHashes
			);
			// set the new default hashes
			DefaultHashes::<T, I>::put(default_hashes);
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn two() -> T::LeafIndex {
		let two: T::LeafIndex = {
			let one: T::LeafIndex = One::one();
			one.saturating_add(One::one())
		};

		two
	}
}

impl<T: Config<I>, I: 'static> TreeInterface<T::AccountId, T::TreeId, T::Element> for Pallet<T, I> {
	fn create(creator: T::AccountId, depth: u8) -> Result<T::TreeId, DispatchError> {
		// Setting the next tree id
		let tree_id = Self::next_tree_id();
		NextTreeId::<T, I>::mutate(|id| *id += One::one());
		// get unit of two
		let two: T::LeafIndex = Self::two();
		// get default edge nodes
		let num_of_zero_nodes = depth;
		let default_edge_nodes = Self::default_hashes()
			.into_iter()
			.take(num_of_zero_nodes as _)
			.collect();
		// Setting up the tree
		let tree_metadata = TreeMetadata {
			creator,
			depth,
			paused: false,
			max_leaves: two.saturating_pow(depth.into()),
			leaf_count: T::LeafIndex::zero(),
			root: T::Element::default(),
			edge_nodes: default_edge_nodes,
		};

		Trees::<T, I>::insert(tree_id, tree_metadata);
		Ok(tree_id)
	}

	fn insert_in_order(id: T::TreeId, leaf: T::Element) -> Result<T::Element, DispatchError> {
		let tree = Trees::<T, I>::get(id);
		let default_hashes = DefaultHashes::<T, I>::get();
		let mut edge_index = tree.leaf_count;
		let mut hash = leaf;
		let mut edge_nodes = tree.edge_nodes.clone();
		// Update the tree
		let two = Self::two();
		for i in 0..edge_nodes.len() {
			hash = if edge_index % two == Zero::zero() {
				edge_nodes[i] = hash;
				let h = T::Hasher::hash_two(&hash.to_bytes(), &default_hashes[i].to_bytes())?;
				T::Element::from_vec(h)
			} else {
				let h = T::Hasher::hash_two(&edge_nodes[i].to_bytes(), &hash.to_bytes())?;
				T::Element::from_vec(h)
			};

			edge_index /= two;
		}

		Leaves::<T, I>::insert(id, tree.leaf_count, leaf);
		Trees::<T, I>::insert(id, TreeMetadata {
			creator: tree.creator,
			depth: tree.depth,
			paused: tree.paused,
			max_leaves: tree.max_leaves,
			leaf_count: tree.leaf_count + One::one(),
			root: hash,
			edge_nodes,
		});

		// Setting the next root index
		let root_index = Self::next_root_index();
		NextRootIndex::<T, I>::mutate(|i| *i = i.saturating_add(One::one()) % T::RootHistorySize::get());
		CachedRoots::<T, I>::insert(id, root_index, hash);
		NextLeafIndex::<T, I>::mutate(id, |i| *i += One::one());

		// return the root
		Ok(hash)
	}
}

impl<T: Config<I>, I: 'static> TreeInspector<T::AccountId, T::TreeId, T::Element> for Pallet<T, I> {
	fn get_root(tree_id: T::TreeId) -> Result<T::Element, DispatchError> {
		ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
		Ok(Trees::<T, I>::get(tree_id).root)
	}

	fn is_known_root(tree_id: T::TreeId, target_root: T::Element) -> Result<bool, DispatchError> {
		ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
		let mut temp: T::RootIndex = Zero::zero();
		while temp < T::RootHistorySize::get() {
			let cached_root = CachedRoots::<T, I>::get(tree_id, temp);
			if cached_root == target_root {
				return Ok(true);
			}

			temp += One::one();
		}

		Ok(false)
	}
}
