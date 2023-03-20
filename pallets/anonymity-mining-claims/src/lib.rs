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

mod benchmarking;
mod benchmarking_utils;

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
use pallet_vanchor::VAnchorConfiguration;
use sp_runtime::traits::Zero;
use sp_std::{convert::TryInto, vec};
use webb_primitives::{
	anonymity_mining::RewardProofData,
	linkable_tree::LinkableTreeInterface,
	traits::vanchor::{VAnchorInspector, VAnchorInterface},
	verifier::ClaimsVerifierModule,
	webb_proposals::ResourceId,
	ElementTrait,
};

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
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

		// /// Proposal nonce type
		// type ProposalNonce: Encode
		// 	+ Decode
		// 	+ Parameter
		// 	+ AtLeast32Bit
		// 	+ Default
		// 	+ Copy
		// 	+ MaxEncodedLen
		// 	+ From<Self::LeafIndex>
		// 	+ Into<Self::LeafIndex>;

		/// History size of roots for each unspent tree
		type UnspentRootHistorySize: Get<Self::RootIndex>;

		/// History size of roots for each spent tree
		type SpentRootHistorySize: Get<Self::RootIndex>;

		/// Currency type for taking unspents
		type Currency: MultiCurrency<Self::AccountId>;

		/// VAnchor Interface
		type VAnchor: VAnchorInterface<VAnchorConfiguration<Self, I>>
			+ VAnchorInspector<VAnchorConfiguration<Self, I>>;

		/// The verifier
		type ClaimsVerifier: ClaimsVerifierModule;

		/// The max number of anchors supported by the pallet
		type MaxAnchors: Get<u32>;

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

	/// The proposal nonce used to prevent replay attacks on execute_proposal
	#[pallet::storage]
	#[pallet::getter(fn proposal_nonce)]
	pub type ProposalNonce<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::ProposalNonce, ValueQuery>;

	/// The number of anchors (size) of the bridge where rewards are being issued from.
	#[pallet::storage]
	#[pallet::getter(fn number_of_anchors)]
	pub type NumberOfAnchors<T: Config<I>, I: 'static = ()> = StorageValue<_, u8, ValueQuery>;

	/// FIX: A map for each resourceID that has been registered in the pallet
	#[pallet::storage]
	#[pallet::getter(fn registered_resource_ids)]
	pub type RegisteredResourceIds<T: Config<I>, I: 'static = ()> =
		StorageValue<_, BoundedVec<ResourceId, T::MaxAnchors>, ValueQuery>;

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
		pub number_of_anchors: u8,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default(), number_of_anchors: 0 }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			NumberOfAnchors::<T, I>::put(self.number_of_anchors);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		AnchorAdded,
		AnchorUpdated,
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
		/// Invalid resource ID length. Needs to be equal to number_of_anchors
		InvalidResourceIdLength,
		/// Resource ID already initialized
		ResourceIdAlreadyInitialized,
		/// Cannot register more resource_ids. List is full.
		ResourceIdListIsFull,
		/// Invalid unspent roots. Need to update history for the resource_ids
		InvalidUnspentRoots,
		/// Invalid unspent roots array length. Needs to be equal to number_of_anchors
		InvalidUnspentRootsLength,
		/// Invalid spent roots. Need to update history for the resource_ids
		InvalidSpentRoots,
		/// Invalid spent roots array length. Needs to be equal to number_of_anchors
		InvalidSpentRootsLength,
		InvalidNumberOfAnchors
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn create(
			origin: OriginFor<T>,
			number_of_anchors: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			ensure!((number_of_anchors == 2) || (number_of_anchors == 8), Error::<T, I>::InvalidNumberOfAnchors);
			let depth = 30;
			let nonce = ProposalNonce::<T, I>::get().saturating_add(T::ProposalNonce::from(1u32));
			// Get the resource IDs and populate the full array of resources including the default
			// ones
			Self::_create(None, depth, number_of_anchors, asset, nonce)?;
			Ok(().into())
		}
		#[pallet::weight(1)]
		#[pallet::call_index(2)]
		pub fn claim(
			origin: OriginFor<T>,
			reward_proof_data: RewardProofData<T::Element>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			// Get the resource IDs and populate the full array of resources including the default
			// ones
			let resource_ids = Self::registered_resource_ids();
			let mut resource_id_vec = resource_ids.to_vec();
			let number_of_anchors = Self::number_of_anchors();
			if resource_ids.len() != number_of_anchors as usize {
				resource_id_vec.resize(number_of_anchors.into(), ResourceId::default());
			}
			// Claim anonymity points
			Self::claim_ap(T::APVanchorTreeId::get(), reward_proof_data, resource_id_vec)?;
			Ok(().into())
		}
	}
}
pub type RootIndex = u32;

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn _create(
		creator: Option<T::AccountId>,
		depth: u8,
		number_of_anchors: u8,
		asset: CurrencyIdOf<T, I>,
		nonce: T::ProposalNonce,
	) -> Result<T::TreeId, DispatchError> {
		// Nonce should be greater than the proposal nonce in storage
		let id = T::VAnchor::create(creator, depth, number_of_anchors as u32, asset, nonce)?;

		// set number_of_anchors value
		NumberOfAnchors::<T, I>::mutate(|i| *i = number_of_anchors);

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
		let number_of_anchors = Self::number_of_anchors();
		ensure!(
			proof_data.spent_roots.len() == number_of_anchors as usize,
			Error::<T, I>::InvalidSpentRootsLength
		);
		ensure!(
			proof_data.unspent_roots.len() == number_of_anchors as usize,
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
		let res = T::ClaimsVerifier::verify(&bytes, &proof_data.proof, number_of_anchors)?;
		ensure!(res, Error::<T, I>::InvalidProof);
		Ok(())
	}
	// / Update unspent root
	// #[allow(dead_code)]
	fn _init_resource_id_history(
		resource_id: ResourceId,
		unspent_root: T::Element,
		spent_root: T::Element,
	) -> Result<(), DispatchError> {
		let _number_of_anchors = Self::number_of_anchors();
		let registered_resource_ids = Self::registered_resource_ids();
		ensure!(
			!registered_resource_ids.contains(&resource_id),
			Error::<T, I>::ResourceIdAlreadyInitialized
		);
		ensure!(
			registered_resource_ids.len() < Self::number_of_anchors().into(),
			Error::<T, I>::ResourceIdListIsFull
		);
		// Add resource_id to list
		let mut registered_resource_id_vec = registered_resource_ids;
		registered_resource_id_vec
			.try_push(resource_id)
			.map_err(|_| Error::<T, I>::ResourceIdListIsFull)?;
		RegisteredResourceIds::<T, I>::put(registered_resource_id_vec);

		// Add unspent root
		let root_index: T::RootIndex = Zero::zero();
		CachedUnspentRoots::<T, I>::insert(resource_id, root_index, unspent_root);
		// Add spent root
		CachedSpentRoots::<T, I>::insert(resource_id, root_index, spent_root);

		NextUnspentRootIndex::<T, I>::mutate(resource_id, |i| *i = One::one());
		NextSpentRootIndex::<T, I>::mutate(resource_id, |i| *i = One::one());

		Ok(())
	}

	/// Update unspent root
	// #[allow(dead_code)]
	fn _update_unspent_root(
		resource_id: ResourceId,
		unspent_root: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(!Self::get_unspent_roots(resource_id).is_empty(), Error::<T, I>::InvalidResourceId);

		// Update unspent root
		let root_index = Self::next_unspent_root_index(resource_id);
		CachedUnspentRoots::<T, I>::insert(resource_id, root_index, unspent_root);

		NextUnspentRootIndex::<T, I>::mutate(resource_id, |i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});
		Ok(())
	}

	/// Update spent root
	// #[allow(dead_code)]
	fn _update_spent_root(
		resource_id: ResourceId,
		spent_root: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(!Self::get_spent_roots(resource_id).is_empty(), Error::<T, I>::InvalidResourceId);
		// Update spent root
		let root_index = Self::next_spent_root_index(resource_id);
		CachedSpentRoots::<T, I>::insert(resource_id, root_index, spent_root);

		NextSpentRootIndex::<T, I>::mutate(resource_id, |i| {
			*i = i.saturating_add(One::one()) % T::UnspentRootHistorySize::get()
		});

		Ok(())
	}

	/// Ensure valid unspent roots
	/// - unspent_roots and resource_ids must have the same length
	/// - unspent_roots must be in resource_id root history
	/// - resource_ids must be registered
	fn ensure_valid_unspent_roots(
		resource_ids: &Vec<ResourceId>,
		unspent_roots: &Vec<T::Element>,
	) -> Result<(), DispatchError> {
		let number_of_anchors = NumberOfAnchors::<T, I>::get();

		// Validate size of arrays provided
		ensure!(
			unspent_roots.len() as u8 == number_of_anchors,
			Error::<T, I>::InvalidUnspentRootsLength
		);
		ensure!(
			resource_ids.len() as u8 == number_of_anchors,
			Error::<T, I>::InvalidResourceIdLength
		);

		// Validate if unspent_roots are in resource_id root history
		for (resource_id, root) in resource_ids.iter().zip(unspent_roots) {
			let historical_roots = Self::get_unspent_roots(*resource_id);
			ensure!(!historical_roots.is_empty(), Error::<T, I>::InvalidResourceId);
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
		let number_of_anchors = NumberOfAnchors::<T, I>::get();

		// validate size of arrays provided
		ensure!(
			spent_roots.len() as u8 == number_of_anchors,
			Error::<T, I>::InvalidSpentRootsLength
		);
		ensure!(
			resource_ids.len() as u8 == number_of_anchors,
			Error::<T, I>::InvalidResourceIdLength
		);

		// validate if spent_roots are in resource_id root history
		for (resource_id, root) in resource_ids.iter().zip(spent_roots) {
			let historical_roots = Self::get_spent_roots(*resource_id);
			ensure!(!historical_roots.is_empty(), Error::<T, I>::InvalidResourceId);
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

	/// Handle claiming of AP tokens
	pub fn claim_ap(
		id: T::TreeId,
		reward_proof_data: RewardProofData<T::Element>,
		resource_ids: Vec<ResourceId>,
	) -> DispatchResultWithPostInfo {
		// Check if nullifier has been spent
		let is_spent = RewardNullifierHashes::<T, I>::get(reward_proof_data.reward_nullifier);

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
