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

//! # Anonymity Mining Claims Module
//!
//! ## Overview
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	ensure,
	pallet_prelude::DispatchError,
	sp_runtime::{
		traits::{AccountIdConversion, One, Saturating},
		FixedI64, FixedPointNumber, SaturatedConversion,
	},
	traits::{Get, Time},
	PalletId,
};
use orml_traits::MultiCurrency;
pub use pallet::*;
use pallet_vanchor::VAnchorConfigration;
use sp_std::{convert::TryInto, vec};
use webb_primitives::{
	linkable_tree::{LinkableTreeInspector, LinkableTreeInterface},
	traits::{
		linkable_tree::*,
		merkle_tree::*,
		vanchor::{VAnchorInspector, VAnchorInterface},
	},
	types::vanchor::{ExtData, ProofData, VAnchorMetadata},
	utils::compute_chain_id_type,
	webb_proposals::{ResourceId, TargetSystem},
	ElementTrait,
};

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> = <<T as pallet::Config<I>>::Currency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]

	pub struct Pallet<T, I = ()>(_);

	#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
	pub struct RewardProofData<E> {
		pub rate: u8,
		pub fee: u8,
		pub note_ak_alpha_x: E,
		pub note_ak_alpha_y: E,
		pub ext_data_hash: E,
		pub input_root: E,
		pub input_nullifier: E,
		pub output_commitment: E,
		pub spent_roots: Vec<E>,
		pub unspent_roots: Vec<E>,
	}

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_balances::Config + pallet_vanchor::Config<I>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Account Identifier from which the internal Pot is generated.
		type PotId: Get<PalletId>;

		/// AP Vanchor Tree Id
		type APVanchorTreeId: Get<Self::TreeId>;

		/// History size of roots for each unspent tree
		type UnspentRootHistorySize: Get<Self::RootIndex>;

		/// History size of roots for each spent tree
		type SpentRootHistorySize: Get<Self::RootIndex>;

		/// Currency type for taking unspents
		type Currency: MultiCurrency<Self::AccountId>;

		/// VAnchor Interface
		type VAnchor: VAnchorInterface<VAnchorConfigration<Self, I>>
			+ VAnchorInspector<VAnchorConfigration<Self, I>>;

		/// AP asset id
		#[pallet::constant]
		type AnonymityPointsAssetId: Get<CurrencyIdOf<Self, I>>;

		/// Reward asset id
		#[pallet::constant]
		type RewardAssetId: Get<CurrencyIdOf<Self, I>>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// A helper map for denoting whether an tree is bridged to given chain
	#[pallet::storage]
	#[pallet::getter(fn linkable_tree_has_edge)]
	pub type LinkableTreeHasEdge<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::TreeId, T::ChainId), bool, ValueQuery>;

	/// The next unspent tree root index
	#[pallet::storage]
	#[pallet::getter(fn next_unspent_root_index)]
	pub(super) type NextUnspentRootIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::RootIndex, ValueQuery>;

	/// The next spent tree root index
	#[pallet::storage]
	#[pallet::getter(fn next_spent_root_index)]
	pub(super) type NextSpentRootIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::RootIndex, ValueQuery>;

	// Map of treeId -> root index -> root value for unspent roots
	#[pallet::storage]
	#[pallet::getter(fn cached_unspent_roots)]
	pub type CachedUnspentRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
		ValueQuery,
	>;

	// Map of treeId -> root index -> root value for spent roots
	#[pallet::storage]
	#[pallet::getter(fn cached_spent_roots)]
	pub type CachedSpentRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
		ValueQuery,
	>;

	/// The map of spent reward_nullfiier_hashes
	#[pallet::storage]
	#[pallet::getter(fn reward_nullifier_hashes)]
	pub type RewardNullifierHashes<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::Element, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: (PhantomData<T>, PhantomData<I>),
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {}
	}

	#[pallet::storage]
	#[pallet::getter(fn get_pool_weight)]
	pub type PoolWeight<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tokens_sold)]
	pub type TokensSold<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		UpdatedPoolWeight { pool_weight: u64 },
		UpdatedTokensSold { tokens_sold: u64 },
		AnchorEdgeAdded,
		AnchorEdgeUpdated,
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Error during hashing
		HashError,
		/// Invalid resource ID
		InvalidResourceId,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		#[pallet::call_index(0)]
		pub fn claim(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			amount: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	// Add rewardNullifier hash
	fn add_reward_nullifier_hash(nullifier_hash: T::Element) -> Result<(), DispatchError> {
		RewardNullifierHashes::<T, I>::insert(nullifier_hash, true);
		Ok(())
	}

	/// Handle proof verification
	pub fn handle_proof_verification(
		proof_data: &ProofData<T::Element>,
	) -> Result<(), DispatchError> {
		// Stubbed out for now
		Ok(())
	}

	/// Update unspent root
	fn update_unspent_root(
		unspent_tree_id: T::TreeId,
		unspent_root_index: T::RootIndex,
		unspent_root: T::Element,
	) {
		// Update unspent root
		CachedUnspentRoots::<T, I>::insert(unspent_tree_id, unspent_root_index, unspent_root);
	}

	/// Update spent root
	fn update_spent_root(
		spent_tree_id: T::TreeId,
		spent_root_index: T::RootIndex,
		spent_root: T::Element,
	) {
		// Update spent root
		CachedSpentRoots::<T, I>::insert(spent_tree_id, spent_root_index, spent_root);
	}

	/// Handle claiming of AP tokens
	pub fn claim_ap(
		unspent_resource_id: ResourceId,
		spent_resource_id: ResourceId,
		recipient: T::AccountId,
		amount: BalanceOf<T, I>,
		merkle_root: T::Element,
		latest_leaf_index: T::LeafIndex,
		proof_data: ProofData<T::Element>,
		reward_proof_data: RewardProofData<T::Element>,
		unspent_root: T::Element,
		spent_root: T::Element,
		reward_nullifier_hash: T::Element,
	) -> DispatchResultWithPostInfo {
		// Handle proof verification
		Self::handle_proof_verification(&proof_data)?;

		let unspent_tree_id: T::TreeId = match unspent_resource_id.target_system() {
			TargetSystem::Substrate(system) => system.tree_id.into(),
			_ => {
				ensure!(false, Error::<T, I>::InvalidResourceId);
				T::TreeId::default()
			},
		};

		let spent_tree_id: T::TreeId = match spent_resource_id.target_system() {
			TargetSystem::Substrate(system) => system.tree_id.into(),
			_ => {
				ensure!(false, Error::<T, I>::InvalidResourceId);
				T::TreeId::default()
			},
		};

		let unspent_root_index = Self::next_unspent_root_index();
		let spent_root_index = Self::next_spent_root_index();

		NextUnspentRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});
		NextSpentRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});

		// Add reward_nullifier_hash
		Self::add_reward_nullifier_hash(reward_nullifier_hash);

		// Add nullifier on VAnchor
		for nullifier in &proof_data.input_nullifiers {
			T::VAnchor::add_nullifier_hash(Self::ap_vanchor_tree_id(), *nullifier)?;
		}

		// Insert output commitments into the AP VAnchor
		for comm in &proof_data.output_commitments {
			T::LinkableTree::insert_in_order(Self::ap_vanchor_tree_id(), *comm)?;
		}

		Ok(().into())
	}

	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}

	/// Get AP Vanchor tree id
	pub fn ap_vanchor_tree_id() -> T::TreeId {
		T::APVanchorTreeId::get()
	}
}
