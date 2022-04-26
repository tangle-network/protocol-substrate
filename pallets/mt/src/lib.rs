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

use frame_support::traits::{Currency, Get, ReservableCurrency};
use frame_system::Config as SystemConfig;
use sp_runtime::traits::{AtLeast32Bit, One, Saturating, Zero};
use sp_std::prelude::*;
use webb_primitives::{
	hasher::*,
	traits::merkle_tree::{TreeInspector, TreeInterface},
	types::{DepositDetails, ElementTrait},
};

type DepositBalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

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

		/// The index for the default merkle root
		type DefaultMerkleRootIndex: Get<u8>;
	}

	#[pallet::storage]
	#[pallet::getter(fn existing_deposit)]
	/// Details of the module's parameters
	pub(super) type Deposit<T: Config<I>, I: 'static = ()> =
		StorageValue<_, DepositDetails<T::AccountId, DepositBalanceOf<T, I>>, OptionQuery>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_tree_id)]
	pub(super) type NextTreeId<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::TreeId, ValueQuery>;

	/// The map of trees to their metadata
	#[pallet::storage]
	#[pallet::getter(fn trees)]
	pub type Trees<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		TreeMetadata<T::AccountId, T::LeafIndex, T::Element>,
		OptionQuery,
	>;

	/// The default hashes for this tree pallet
	#[pallet::storage]
	#[pallet::getter(fn default_hashes)]
	pub(super) type DefaultHashes<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Vec<T::Element>, ValueQuery>;

	/// The map of (tree_id, index) to the leaf commitment
	#[pallet::storage]
	#[pallet::getter(fn leaves)]
	pub(super) type Leaves<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::LeafIndex,
		T::Element,
		ValueQuery,
	>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_root_index)]
	pub(super) type NextRootIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::RootIndex, ValueQuery>;

	/// The next tree identifier up for grabs
	#[pallet::storage]
	#[pallet::getter(fn next_leaf_index)]
	pub(super) type NextLeafIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, T::LeafIndex, ValueQuery>;

	/// Map of root history from tree id to root index to root values
	#[pallet::storage]
	#[pallet::getter(fn cached_roots)]
	pub(super) type CachedRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New tree created
		TreeCreation { tree_id: T::TreeId, who: T::AccountId },
		/// New leaf inserted
		LeafInsertion { tree_id: T::TreeId, leaf_index: T::LeafIndex, leaf: T::Element },
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
		/// Tree doesnt exist
		ZeroRootIndexDoesntExist,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			if Self::is_default_hashes_empty() {
				let temp_hashes = generate_default_hashes::<T, I>();
				DefaultHashes::<T, I>::put(temp_hashes);
			}
			1u64 + 1u64
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: PhantomData<T>,
		pub default_hashes: Option<Vec<T::Element>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default(), default_hashes: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			if let Some(default_hashes) = &self.default_hashes {
				DefaultHashes::<T, I>::put(default_hashes);
				return
			}

			let default_hashes = generate_default_hashes::<T, I>();
			DefaultHashes::<T, I>::put(default_hashes);
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::create(*depth as u32))]
		pub fn create(origin: OriginFor<T>, depth: u8) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			ensure!(depth <= T::MaxTreeDepth::get() && depth > 0, Error::<T, I>::InvalidTreeDepth);
			// calculate the deposit, we charge the user based on # of leaves
			let deposit = T::DataDepositPerByte::get()
				.saturating_mul(T::Two::get().saturating_pow(depth.into()))
				.saturating_add(T::DataDepositBase::get());

			T::Currency::reserve(&origin, deposit)?;

			let tree_id = <Self as TreeInterface<_, _, _>>::create(Some(origin.clone()), depth)?;

			Self::deposit_event(Event::TreeCreation { tree_id, who: origin });
			Ok(().into())
		}

		#[pallet::weight(T::WeightInfo::insert())]
		pub fn insert(
			origin: OriginFor<T>,
			tree_id: T::TreeId,
			leaf: T::Element,
		) -> DispatchResultWithPostInfo {
			let _origin = ensure_signed(origin)?;
			ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
			let tree = Self::get_tree(tree_id)?;
			let next_index = Self::next_leaf_index(tree_id);
			ensure!(next_index == tree.leaf_count, Error::<T, I>::InvalidLeafIndex);
			ensure!(
				tree.leaf_count.saturating_add(One::one()) <= tree.max_leaves,
				Error::<T, I>::ExceedsMaxLeaves
			);
			// insert the leaf
			<Self as TreeInterface<_, _, _>>::insert_in_order(tree_id, leaf)?;

			Self::deposit_event(Event::LeafInsertion { tree_id, leaf_index: next_index, leaf });

			Ok(().into())
		}

		#[pallet::weight(T::WeightInfo::force_set_default_hashes(default_hashes.len() as u32))]
		pub fn force_set_default_hashes(
			origin: OriginFor<T>,
			default_hashes: Vec<T::Element>,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			let len_of_hashes = default_hashes.len();
			ensure!(
				len_of_hashes > 0 && len_of_hashes <= T::MaxTreeDepth::get() as usize,
				Error::<T, I>::ExceedsMaxDefaultHashes
			);
			// set the new default hashes
			DefaultHashes::<T, I>::put(default_hashes);
			Ok(().into())
		}
	}

	pub fn generate_default_hashes<T: Config<I>, I: 'static>() -> Vec<T::Element> {
		let mut temp_hashes: Vec<T::Element> = Vec::with_capacity(T::MaxTreeDepth::get() as usize);
		let default_zero = T::DefaultZeroElement::get();
		temp_hashes.push(default_zero);
		let mut temp_hash = default_zero.to_bytes().to_vec();
		for _ in 0..T::MaxTreeDepth::get() {
			temp_hash = T::Hasher::hash_two(&temp_hash, &temp_hash).unwrap();
			temp_hashes.push(T::Element::from_vec(temp_hash.clone()));
		}

		temp_hashes
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

	fn is_default_hashes_empty() -> bool {
		let default_hashes = Self::default_hashes();
		default_hashes.is_empty()
	}

	fn get_tree(
		tree_id: T::TreeId,
	) -> Result<TreeMetadata<T::AccountId, T::LeafIndex, T::Element>, DispatchError> {
		let tree = Trees::<T, I>::get(tree_id);
		ensure!(tree.is_some(), Error::<T, I>::TreeDoesntExist);
		Ok(tree.unwrap())
	}
}

impl<T: Config<I>, I: 'static> TreeInterface<T::AccountId, T::TreeId, T::Element> for Pallet<T, I> {
	fn create(creator: Option<T::AccountId>, depth: u8) -> Result<T::TreeId, DispatchError> {
		// Setting the next tree id
		let tree_id = Self::next_tree_id();
		NextTreeId::<T, I>::mutate(|id| *id += One::one());
		// get unit of two
		let two: T::LeafIndex = Self::two();
		// get default edge nodes
		let num_of_zero_nodes = depth;
		let default_edge_nodes =
			Self::default_hashes().into_iter().take(num_of_zero_nodes as _).collect();
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
		let tree = Self::get_tree(id)?;
		let default_hashes = DefaultHashes::<T, I>::get();
		let mut edge_index = tree.leaf_count;
		let mut hash = leaf;
		let mut edge_nodes = tree.edge_nodes.clone();
		// Update the tree
		let two = Self::two();
		for i in 0..edge_nodes.len() {
			hash = if edge_index % two == Zero::zero() {
				edge_nodes[i] = hash;
				let h = T::Hasher::hash_two(hash.to_bytes(), default_hashes[i].to_bytes())?;
				T::Element::from_vec(h)
			} else {
				let h = T::Hasher::hash_two(edge_nodes[i].to_bytes(), hash.to_bytes())?;
				T::Element::from_vec(h)
			};

			edge_index /= two;
		}

		Leaves::<T, I>::insert(id, tree.leaf_count, leaf);
		Trees::<T, I>::insert(
			id,
			TreeMetadata {
				creator: tree.creator,
				depth: tree.depth,
				paused: tree.paused,
				max_leaves: tree.max_leaves,
				leaf_count: tree.leaf_count + One::one(),
				root: hash,
				edge_nodes,
			},
		);

		// Setting the next root index
		let root_index = Self::next_root_index();
		NextRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::RootHistorySize::get()
		});
		CachedRoots::<T, I>::insert(id, root_index, hash);
		NextLeafIndex::<T, I>::mutate(id, |i| *i += One::one());

		// return the root
		Ok(hash)
	}

	fn zero_root(i: u8) -> Result<[u8; 32], DispatchError> {
		ensure!(i < 31, Error::<T, I>::ZeroRootIndexDoesntExist);
		let mut bytes = [0u8; 32];
		if i == 0 {
			hex::decode_to_slice("2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c", &mut bytes as &mut [u8]);
		} else if i == 1 {
			hex::decode_to_slice("13e37f2d6cb86c78ccc1788607c2b199788c6bb0a615a21f2e7a8e88384222f8", &mut bytes as &mut [u8]);
		} else if i == 2 {
			hex::decode_to_slice("217126fa352c326896e8c2803eec8fd63ad50cf65edfef27a41a9e32dc622765", &mut bytes as &mut [u8]);
		} else if i == 3 {
			hex::decode_to_slice("0e28a61a9b3e91007d5a9e3ada18e1b24d6d230c618388ee5df34cacd7397eee", &mut bytes as &mut [u8]);
		}else if i == 4 {
			hex::decode_to_slice("27953447a6979839536badc5425ed15fadb0e292e9bc36f92f0aa5cfa5013587", &mut bytes as &mut [u8]);
		} else if i == 5 {
			hex::decode_to_slice("194191edbfb91d10f6a7afd315f33095410c7801c47175c2df6dc2cce0e3affc", &mut bytes as &mut [u8]);
		} else if i == 6 {
			hex::decode_to_slice("1733dece17d71190516dbaf1927936fa643dc7079fc0cc731de9d6845a47741f", &mut bytes as &mut [u8]);
		} else if i == 7 {
			hex::decode_to_slice("267855a7dc75db39d81d17f95d0a7aa572bf5ae19f4db0e84221d2b2ef999219", &mut bytes as &mut [u8]);
		} else if i == 8 {
			hex::decode_to_slice("1184e11836b4c36ad8238a340ecc0985eeba665327e33e9b0e3641027c27620d", &mut bytes as &mut [u8]);
		} else if i == 9 {
			hex::decode_to_slice("0702ab83a135d7f55350ab1bfaa90babd8fc1d2b3e6a7215381a7b2213d6c5ce", &mut bytes as &mut [u8]);
		} else if i == 10 {
			hex::decode_to_slice("2eecc0de814cfd8c57ce882babb2e30d1da56621aef7a47f3291cffeaec26ad7", &mut bytes as &mut [u8]);
		} else if i == 11 {
			hex::decode_to_slice("280bc02145c155d5833585b6c7b08501055157dd30ce005319621dc462d33b47", &mut bytes as &mut [u8]);
		} else if i == 12 {
			hex::decode_to_slice("045132221d1fa0a7f4aed8acd2cbec1e2189b7732ccb2ec272b9c60f0d5afc5b", &mut bytes as &mut [u8]);
		} else if i == 13 {
			hex::decode_to_slice("27f427ccbf58a44b1270abbe4eda6ba53bd6ac4d88cf1e00a13c4371ce71d366", &mut bytes as &mut [u8]);
		} else if i == 14 {
			hex::decode_to_slice("1617eaae5064f26e8f8a6493ae92bfded7fde71b65df1ca6d5dcec0df70b2cef", &mut bytes as &mut [u8]);
		} else if i == 15 {
			hex::decode_to_slice("20c6b400d0ea1b15435703c31c31ee63ad7ba5c8da66cec2796feacea575abca", &mut bytes as &mut [u8]);
		} else if i == 16 {
			hex::decode_to_slice("09589ddb438723f53a8e57bdada7c5f8ed67e8fece3889a73618732965645eec", &mut bytes as &mut [u8]);
		} else if i == 17 {
			hex::decode_to_slice("0064b6a738a5ff537db7b220f3394f0ecbd35bfd355c5425dc1166bf3236079b", &mut bytes as &mut [u8]);
		} else if i == 18 {
			hex::decode_to_slice("095de56281b1d5055e897c3574ff790d5ee81dbc5df784ad2d67795e557c9e9f", &mut bytes as &mut [u8]);
		} else if i == 19 {
			hex::decode_to_slice("11cf2e2887aa21963a6ec14289183efe4d4c60f14ecd3d6fe0beebdf855a9b63", &mut bytes as &mut [u8]);
		} else if i == 20 {
			hex::decode_to_slice("2b0f6fc0179fa65b6f73627c0e1e84c7374d2eaec44c9a48f2571393ea77bcbb", &mut bytes as &mut [u8]);
		} else if i == 21 {
			hex::decode_to_slice("16fdb637c2abf9c0f988dbf2fd64258c46fb6a273d537b2cf1603ea460b13279", &mut bytes as &mut [u8]);
		} else if i == 22 {
			hex::decode_to_slice("21bbd7e944f6124dad4c376df9cc12e7ca66e47dff703ff7cedb1a454edcf0ff", &mut bytes as &mut [u8]);
		} else if i == 23 {
			hex::decode_to_slice("2784f8220b1c963e468f590f137baaa1625b3b92a27ad9b6e84eb0d3454d9962", &mut bytes as &mut [u8]);
		} else if i == 24 {
			hex::decode_to_slice("16ace1a65b7534142f8cc1aad810b3d6a7a74ca905d9c275cb98ba57e509fc10", &mut bytes as &mut [u8]);
		} else if i == 25 {
			hex::decode_to_slice("2328068c6a8c24265124debd8fe10d3f29f0665ea725a65e3638f6192a96a013", &mut bytes as &mut [u8]);
		} else if i == 26 {
			hex::decode_to_slice("2ddb991be1f028022411b4c4d2c22043e5e751c120736f00adf54acab1c9ac14", &mut bytes as &mut [u8]);
		} else if i == 27 {
			hex::decode_to_slice("0113798410eaeb95056a464f70521eb58377c0155f2fe518a5594d38cc209cc0", &mut bytes as &mut [u8]);
		} else if i == 28 {
			hex::decode_to_slice("202d1ae61526f0d0d01ef80fb5d4055a7af45721024c2c24cffd6a3798f54d50", &mut bytes as &mut [u8]);
		} else if i == 29 {
			hex::decode_to_slice("23ab323453748129f2765f79615022f5bebd6f4096a796300aab049a60b0f187", &mut bytes as &mut [u8]);
		} else if i == 30 {
			hex::decode_to_slice("1f15585f8947e378bcf8bd918716799da909acdb944c57150b1eb4565fda8aa0", &mut bytes as &mut [u8]);
		} else if i == 31 {
			hex::decode_to_slice("1eb064b21055ac6a350cf41eb30e4ce2cb19680217df3a243617c2838185ad06", &mut bytes as &mut [u8]);
		}

		Ok(bytes)
	}
}

impl<T: Config<I>, I: 'static> TreeInspector<T::AccountId, T::TreeId, T::Element> for Pallet<T, I> {
	fn get_root(tree_id: T::TreeId) -> Result<T::Element, DispatchError> {
		ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
		Ok(Self::get_tree(tree_id)?.root)
	}

	fn is_known_root(tree_id: T::TreeId, target_root: T::Element) -> Result<bool, DispatchError> {
		ensure!(Trees::<T, I>::contains_key(tree_id), Error::<T, I>::TreeDoesntExist);
		let mut temp: T::RootIndex = Zero::zero();
		while temp < T::RootHistorySize::get() {
			let cached_root = CachedRoots::<T, I>::get(tree_id, temp);
			if cached_root == target_root {
				return Ok(true)
			}

			temp += One::one();
		}

		Ok(false)
	}
}
