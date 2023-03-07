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
	webb_proposals::{ResourceId, TargetSystem, TypedChainId},
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
	use pallet_vanchor::ProposalNonce;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]

	pub struct Pallet<T, I = ()>(_);

	#[derive(Clone, Encode, Decode, Debug, Default,	Eq, PartialEq, TypeInfo)]
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

	/// The map of trees to the maximum number of anchor edges they can have
	#[pallet::storage]
	#[pallet::getter(fn max_edges)]
	pub type MaxEdges<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::TreeId, u32, ValueQuery>;

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
		ResourceId,
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
		ResourceId,
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
		fn build(&self) {

		}
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
		/// Reward Nullifier has been used
		RewardNullifierAlreadySpent,
		/// Invalid resource ID
		InvalidResourceId,
		InvalidUnspentRoots,
		InvalidUnspentRootsLength,
		InvalidUnspentChainIds,

		InvalidSpentRoots,
		InvalidSpentRootsLength,
		InvalidSpentChainIds,
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
		proof_data: &RewardProofData<T::Element>,
	) -> Result<(), DispatchError> {
		// Stubbed out for now
		Ok(())
	}

	/// Update unspent root
	fn update_unspent_root(
		unspent_resource_id: ResourceId,
		unspent_root: T::Element,
	) {
		let root_index = Self::next_unspent_root_index();
		// Update unspent root
		CachedUnspentRoots::<T, I>::insert(unspent_resource_id, root_index, unspent_root);

		NextUnspentRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});
	}

	/// Update spent root
	fn update_spent_root(
		spent_resource_id: ResourceId,
		spent_root: T::Element,
	) {
		let root_index = Self::next_spent_root_index();

		// Update spent root
		CachedSpentRoots::<T, I>::insert(spent_resource_id, root_index, spent_root);

		NextSpentRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});
	}

	/// Ensure valid unspent roots
	fn ensure_valid_unspent_roots(
		tree_id: T::TreeId,
		resource_ids: &Vec<ResourceId>,
		unspent_roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		let max_edges = MaxEdges::<T, I>::get(tree_id);
		ensure!(
			unspent_roots.len() as u32 == max_edges,
			Error::<T, I>::InvalidUnspentRootsLength
		);
		ensure!(
			resource_ids.len() as u32 == max_edges,
			Error::<T, I>::InvalidUnspentChainIds
		);
		for (chain_id, root) in resource_ids.iter().zip(unspent_roots) {
			let historical_roots = Self::get_unspent_roots(*chain_id);
			let is_known = historical_roots.contains(root);
			println!("chain_id: {chain_id:?}, root: {root:?}, is_known: {is_known:?}");
			ensure!(
				is_known,
				Error::<T, I>::InvalidUnspentRoots
			);
		}
		Ok(())
	}

	/// Ensure valid spent roots
	fn ensure_valid_spent_roots(
		tree_id: T::TreeId,
		resource_ids: &Vec<ResourceId>,
		spent_roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		let max_edges = MaxEdges::<T, I>::get(tree_id);
		ensure!(
			spent_roots.len() as u32 == max_edges,
			Error::<T, I>::InvalidSpentRootsLength
		);
		ensure!(
			resource_ids.len() as u32 == max_edges,
			Error::<T, I>::InvalidSpentChainIds
		);
		for (resource_id, root) in resource_ids.iter().zip(spent_roots) {
			let historical_roots = Self::get_spent_roots(*resource_id);
			let is_known = historical_roots.contains(root);
			println!("resource_id: {resource_id:?}, root: {root:?}, is_known: {is_known:?}");
			ensure!(
				is_known,
				Error::<T, I>::InvalidSpentRoots
			);
		}
		Ok(())
	}

	/// get unspent roots
	pub fn get_unspent_roots(
		resource_id: ResourceId,
	) -> Vec<T::Element> {
		CachedUnspentRoots::<T, I>::iter_prefix_values(resource_id)
			.into_iter()
			.collect::<Vec<T::Element>>()
	}

	/// get spent roots
	pub fn get_spent_roots(
		resource_id: ResourceId,
	) -> Vec<T::Element> {
		CachedSpentRoots::<T, I>::iter_prefix_values(resource_id)
			.into_iter()
			.collect::<Vec<T::Element>>()

	}

	/// Handle claiming of AP tokens
	pub fn claim_ap(
		id: T::TreeId,
		// unspent_resource_id: ResourceId,
		// spent_resource_id: ResourceId,
		// recipient: T::AccountId,
		// amount: BalanceOf<T, I>,
		// proof_data: ProofData<T::Element>,
		reward_proof_data: RewardProofData<T::Element>,
		unspent_roots: Vec<T::Element>,
		unspent_resource_ids: Vec<ResourceId>,
		spent_roots: Vec<T::Element>,
		spent_resource_ids: Vec<ResourceId>,
		reward_nullifier_hash: T::Element,
	) -> DispatchResultWithPostInfo {
		// Check if nullifier has been spent
		let is_spent = RewardNullifierHashes::<T, I>::get(&reward_nullifier_hash);

		ensure!(
			!is_spent,
			Error::<T, I>::RewardNullifierAlreadySpent
		);

		// Handle proof verification
		Self::handle_proof_verification(&reward_proof_data)?;

		// Check if roots are valid
		Self::ensure_valid_unspent_roots(id, &unspent_resource_ids, &unspent_roots)?;

		Self::ensure_valid_spent_roots(id, &spent_resource_ids, &spent_roots)?;

		// Add reward_nullifier_hash
		Self::add_reward_nullifier_hash(reward_nullifier_hash)?;

		// Add nullifier on VAnchor
		T::VAnchor::add_nullifier_hash(id, reward_proof_data.input_nullifier)?;

		// Insert output commitments into the AP VAnchor
		T::LinkableTree::insert_in_order(id, reward_proof_data.output_commitment)?;

		Ok(().into())
	}

	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}

	// Get AP Vanchor tree id
	// pub fn ap_vanchor_tree_id() -> T::TreeId {
	// 	T::APVanchorTreeId::get()
	// }
}
