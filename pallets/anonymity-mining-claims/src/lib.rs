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

//! # Anonimity mining claims Module
//!
//! A module for anonimity mining rewards connected to the MASP.
//!
//! ## Overview
//!
//! The Anonimity mining claims module provides functionality for the following:
//!
//! * A user can prove that they have a (deposit, withdrawal) pair on the masp and claim anonimity
//! points linked to both the amount that they have deposited and the amount of time they left in
//! the pool.
//!
//! ## Interface
//!
//! ### Permissionless Functions
//!
//! * `claim_ap`: Allows user to prove in zero knowledge they have a pair (spentUTXO, unspentUTXO)
//!   on the MASP
//! 	and accumulate anonimity points following `inputAmount + rate * (spentTimestamp -
//! unspentTimestamp) * amount`

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	ensure,
	pallet_prelude::DispatchError,
	sp_runtime::traits::{AccountIdConversion, One, Saturating},
	traits::Get,
	PalletId,
};
use orml_traits::MultiCurrency;
pub use pallet::*;
use pallet_vanchor::VAnchorConfigration;
use sp_runtime::traits::Zero;
use sp_std::{convert::TryInto, vec};
use webb_primitives::{
	linkable_tree::{LinkableTreeInspector, LinkableTreeInterface},
	traits::vanchor::{VAnchorInspector, VAnchorInterface},
	// types::vanchor::{ExtData, ProofData, VAnchorMetadata},
	// utils::compute_chain_id_type,
	verifier::ClaimsVerifierModule,
	webb_proposals::ResourceId,
	ElementTrait,
};

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
// pub type CurrencyIdOf<T, I> = <<T as pallet::Config<I>>::Currency as MultiCurrency<
// 	<T as frame_system::Config>::AccountId,
// >>::CurrencyId;
pub type CurrencyIdOf<T, I> = <<T as pallet_vanchor::Config<I>>::Currency as MultiCurrency<
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

	#[derive(Clone, Encode, Decode, Debug, Default, Eq, PartialEq, TypeInfo)]
	pub struct RewardProofData<E> {
		pub proof: Vec<u8>,
		pub rate: E,
		pub fee: E,
		pub reward_nullifier: E,
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

		/// The verifier
		type ClaimsVerifier: ClaimsVerifierModule;

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

	/// FIX: The map of trees to the maximum number of anchor edges they can have
	#[pallet::storage]
	#[pallet::getter(fn max_edges)]
	pub type MaxEdges<T: Config<I>, I: 'static = ()> = StorageValue<_, u8, ValueQuery>;

	/// FIX: A map for each resourceID that has been registered in the pallet
	#[pallet::storage]
	#[pallet::getter(fn registered_resource_ids)]
	pub type RegisteredResourceIds<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, u32, ResourceId, ValueQuery>;

	/// Where to register the next resource id on the `RegisteredResourceIds` map
	#[pallet::storage]
	#[pallet::getter(fn next_resource_id_index)]
	pub type NextResourceIdIndex<T: Config<I>, I: 'static = ()> = StorageValue<_, u32, ValueQuery>;

	/// The next unspent tree root index for a specific ResourceId
	#[pallet::storage]
	#[pallet::getter(fn next_unspent_root_index)]
	pub(super) type NextUnspentRootIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::RootIndex, ValueQuery>;

	/// The next spent tree root index for a specific ResourceId
	#[pallet::storage]
	#[pallet::getter(fn next_spent_root_index)]
	pub(super) type NextSpentRootIndex<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, ResourceId, T::RootIndex, ValueQuery>;

	// Map of ResourceId -> root index -> root value for unspent roots
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

	// Map of ResourceId -> root index -> root value for spent roots
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

	/// The map of spent reward_nullfiier_hashes to prevent double claiming
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
		/// Invalid proof
		InvalidProof,
		/// Reward Nullifier has been used
		RewardNullifierAlreadySpent,
		/// Invalid resource ID
		InvalidResourceId,
		/// Invalid resource ID length. Needs to be equal to max_edges
		InvalidResourceIdLength,
		/// Resource ID already initialized
		ResourceIdAlreadyInitialized,
		/// Cannot register more resource_ids. List is full.
		ResourceIdListIsFull,
		/// Invalid unspent roots. Need to update history for the resource_ids
		InvalidUnspentRoots,
		/// Invalid unspent roots array length. Needs to be equal to max_edges
		InvalidUnspentRootsLength,
		/// Invalid spent roots. Need to update history for the resource_ids
		InvalidSpentRoots,
		/// Invalid spent roots array length. Needs to be equal to max_edges
		InvalidSpentRootsLength,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn claim(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			amount: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}
pub type ProposalNonce = u32;
pub type RootIndex = u32;

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn create(
		creator: Option<T::AccountId>,
		depth: u8,
		max_edges: u8,
		asset: CurrencyIdOf<T, I>,
		nonce: u32,
	) -> Result<T::TreeId, DispatchError> {
		// Nonce should be greater than the proposal nonce in storage
		let id = T::VAnchor::create(creator.clone(), depth, max_edges as u32, asset, nonce.into())?;

		// set max_edges value
		MaxEdges::<T, I>::mutate(|i| *i = max_edges);

		Ok(id)
	}
	// Add rewardNullifier hash
	fn add_reward_nullifier_hash(nullifier_hash: T::Element) -> Result<(), DispatchError> {
		RewardNullifierHashes::<T, I>::insert(nullifier_hash, true);
		Ok(())
	}
	/// Handle proof verification
	pub fn handle_proof_verification(
		proof_data: &RewardProofData<T::Element>,
	) -> Result<(), DispatchError> {
		let max_edges = Self::max_edges();
		ensure!(
			proof_data.spent_roots.len() == max_edges as usize,
			Error::<T, I>::InvalidSpentRootsLength
		);
		ensure!(
			proof_data.unspent_roots.len() == max_edges as usize,
			Error::<T, I>::InvalidUnspentRootsLength
		);
		let mut bytes = Vec::new();
		bytes.extend_from_slice(proof_data.rate.to_bytes());
		bytes.extend_from_slice(proof_data.fee.to_bytes());
		bytes.extend_from_slice(proof_data.reward_nullifier.to_bytes());
		bytes.extend_from_slice(proof_data.note_ak_alpha_x.to_bytes());
		bytes.extend_from_slice(proof_data.note_ak_alpha_y.to_bytes());
		bytes.extend_from_slice(proof_data.ext_data_hash.to_bytes());
		bytes.extend_from_slice(proof_data.input_root.to_bytes());
		bytes.extend_from_slice(proof_data.input_nullifier.to_bytes());
		bytes.extend_from_slice(proof_data.output_commitment.to_bytes());
		for root in &proof_data.spent_roots {
			bytes.extend_from_slice(root.to_bytes());
		}
		for root in &proof_data.unspent_roots {
			bytes.extend_from_slice(root.to_bytes());
		}
		// Verify the zero-knowledge proof
		let res = T::ClaimsVerifier::verify(&bytes, &proof_data.proof, max_edges)?;
		ensure!(res, Error::<T, I>::InvalidProof);
		Ok(())
	}
	// / Update unspent root
	fn init_resource_id_history(
		resource_id: ResourceId,
		unspent_root: T::Element,
		spent_root: T::Element,
	) -> Result<(), DispatchError> {
		let max_edges = Self::max_edges();
		let next_resource_id_index = Self::next_resource_id_index();
		ensure!(next_resource_id_index < (max_edges as u32), Error::<T, I>::ResourceIdListIsFull);

		let registered_resource_ids = Self::get_registered_resource_ids();
		ensure!(
			registered_resource_ids.contains(&resource_id) == false,
			Error::<T, I>::ResourceIdAlreadyInitialized
		);
		let resource_id_index = Self::next_resource_id_index();
		ensure!((resource_id_index as u8) < max_edges, Error::<T, I>::ResourceIdListIsFull);

		RegisteredResourceIds::<T, I>::insert(resource_id_index, resource_id);

		NextResourceIdIndex::<T, I>::mutate(|i| *i = i.saturating_add(One::one()));
		let root_index: T::RootIndex = Zero::zero();
		// Add unspent root
		CachedUnspentRoots::<T, I>::insert(resource_id, root_index, unspent_root);
		// Add spent root
		CachedSpentRoots::<T, I>::insert(resource_id, root_index, spent_root);

		NextUnspentRootIndex::<T, I>::mutate(resource_id, |i| *i = One::one());
		NextSpentRootIndex::<T, I>::mutate(resource_id, |i| *i = One::one());

		Ok(())
	}

	/// Update unspent root
	fn update_unspent_root(
		resource_id: ResourceId,
		unspent_root: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(Self::get_unspent_roots(resource_id).len() >= 1, Error::<T, I>::InvalidResourceId);

		// Update unspent root
		let root_index = Self::next_unspent_root_index(resource_id);
		CachedUnspentRoots::<T, I>::insert(resource_id, root_index, unspent_root);

		NextUnspentRootIndex::<T, I>::mutate(resource_id, |i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});
		Ok(())
	}

	/// Update spent root
	fn update_spent_root(
		resource_id: ResourceId,
		spent_root: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(Self::get_spent_roots(resource_id).len() >= 1, Error::<T, I>::InvalidResourceId);
		// Update spent root
		let root_index = Self::next_spent_root_index(resource_id);
		CachedSpentRoots::<T, I>::insert(resource_id, root_index, spent_root);

		NextSpentRootIndex::<T, I>::mutate(resource_id, |i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});

		Ok(())
	}

	/// Ensure valid unspent roots
	fn ensure_valid_unspent_roots(
		resource_ids: &Vec<ResourceId>,
		unspent_roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		let max_edges = MaxEdges::<T, I>::get();

		// validate size of arrays provided
		ensure!(unspent_roots.len() as u8 == max_edges, Error::<T, I>::InvalidUnspentRootsLength);
		ensure!(resource_ids.len() as u8 == max_edges, Error::<T, I>::InvalidResourceIdLength);

		// validate if unspent_roots are in resource_id root history
		for (resource_id, root) in resource_ids.iter().zip(unspent_roots) {
			let historical_roots = Self::get_unspent_roots(*resource_id);
			ensure!(historical_roots.len() >= 1, Error::<T, I>::InvalidResourceId);
			let is_known = historical_roots.contains(root);
			ensure!(is_known, Error::<T, I>::InvalidUnspentRoots);
		}
		Ok(())
	}

	/// Ensure valid spent roots
	fn ensure_valid_spent_roots(
		resource_ids: &Vec<ResourceId>,
		spent_roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		let max_edges = MaxEdges::<T, I>::get();

		// validate size of arrays provided
		ensure!(spent_roots.len() as u8 == max_edges, Error::<T, I>::InvalidSpentRootsLength);
		ensure!(resource_ids.len() as u8 == max_edges, Error::<T, I>::InvalidResourceIdLength);

		// validate if spent_roots are in resource_id root history
		for (resource_id, root) in resource_ids.iter().zip(spent_roots) {
			let historical_roots = Self::get_spent_roots(*resource_id);
			ensure!(historical_roots.len() >= 1, Error::<T, I>::InvalidResourceId);
			let is_known = historical_roots.contains(root);
			ensure!(is_known, Error::<T, I>::InvalidSpentRoots);
		}
		Ok(())
	}

	/// get unspent roots
	pub fn get_unspent_roots(resource_id: ResourceId) -> Vec<T::Element> {
		CachedUnspentRoots::<T, I>::iter_prefix_values(resource_id)
			.into_iter()
			.collect::<Vec<T::Element>>()
	}

	/// get spent roots
	pub fn get_spent_roots(resource_id: ResourceId) -> Vec<T::Element> {
		CachedSpentRoots::<T, I>::iter_prefix_values(resource_id)
			.into_iter()
			.collect::<Vec<T::Element>>()
	}

	/// get registered resource ids
	pub fn get_registered_resource_ids() -> Vec<ResourceId> {
		RegisteredResourceIds::<T, I>::iter_values().collect::<Vec<ResourceId>>()
	}

	/// Handle claiming of AP tokens
	pub fn claim_ap(
		id: T::TreeId,
		reward_proof_data: RewardProofData<T::Element>,
		resource_ids: Vec<ResourceId>,
	) -> DispatchResultWithPostInfo {
		// Check if nullifier has been spent
		let is_spent = RewardNullifierHashes::<T, I>::get(&reward_proof_data.reward_nullifier);

		ensure!(!is_spent, Error::<T, I>::RewardNullifierAlreadySpent);

		// Handle proof verification
		Self::handle_proof_verification(&reward_proof_data)?;

		// Check if unspent roots are valid
		Self::ensure_valid_unspent_roots(&resource_ids, &reward_proof_data.unspent_roots)?;

		// Check if spent roots are valid
		Self::ensure_valid_spent_roots(&resource_ids, &reward_proof_data.spent_roots)?;

		// Add reward_nullifier_hash
		Self::add_reward_nullifier_hash(reward_proof_data.reward_nullifier)?;

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
}
