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

		/// History size of roots for each deposit tree
		type DepositRootHistorySize: Get<Self::RootIndex>;

		/// History size of roots for each withdraw tree
		type WithdrawRootHistorySize: Get<Self::RootIndex>;

		/// Currency type for taking deposits
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

	/// The next deposit tree root index
	#[pallet::storage]
	#[pallet::getter(fn next_deposit_root_index)]
	pub(super) type NextDepositRootIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::RootIndex, ValueQuery>;

	/// The next withdraw tree root index
	#[pallet::storage]
	#[pallet::getter(fn next_withdraw_root_index)]
	pub(super) type NextWithdrawRootIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::RootIndex, ValueQuery>;

	// Map of treeId -> root index -> root value for deposit roots
	#[pallet::storage]
	#[pallet::getter(fn cached_deposit_roots)]
	pub type CachedDepositRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
		ValueQuery,
	>;

	// Map of treeId -> root index -> root value for withdraw roots
	#[pallet::storage]
	#[pallet::getter(fn cached_withdraw_roots)]
	pub type CachedWithdrawRoots<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::RootIndex,
		T::Element,
		ValueQuery,
	>;

	/// The map of deposit trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn deposit_nullifier_hashes)]
	pub type DepositNullifierHashes<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::Element,
		bool,
		ValueQuery,
	>;

	/// The map of withdraw trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn withdraw_nullifier_hashes)]
	pub type WithdrawNullifierHashes<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::Element,
		bool,
		ValueQuery,
	>;

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
	// Add deposit nullifier hash
	fn add_deposit_nullifier_hash(
		id: T::TreeId,
		nullifier_hash: T::Element,
	) -> Result<(), DispatchError> {
		DepositNullifierHashes::<T, I>::insert(id, nullifier_hash, true);
		Ok(())
	}

	// Add withdraw nullifier hash
	fn add_withdraw_nullifier_hash(
		id: T::TreeId,
		nullifier_hash: T::Element,
	) -> Result<(), DispatchError> {
		WithdrawNullifierHashes::<T, I>::insert(id, nullifier_hash, true);
		Ok(())
	}

	/// Store nullifier hashes in deposit and withdraw reward trees
	fn store_nullifier_hashes(
		id: T::TreeId,
		deposit_nullifier_hash: T::Element,
		withdraw_nullifier_hash: T::Element,
	) -> Result<(), DispatchError> {
		Self::add_deposit_nullifier_hash(id, deposit_nullifier_hash)?;
		Self::add_withdraw_nullifier_hash(id, withdraw_nullifier_hash)?;
		Ok(())
	}

	/// Handle proof verification
	pub fn handle_proof_verification(
		proof_data: &ProofData<T::Element>,
	) -> Result<(), DispatchError> {
		// Stubbed out for now
		Ok(())
	}

	/// Update deposit root
	fn update_deposit_root(
		deposit_tree_id: T::TreeId,
		deposit_root_index: T::RootIndex,
		deposit_root: T::Element,
	) {
		// Update deposit root
		CachedDepositRoots::<T, I>::insert(deposit_tree_id, deposit_root_index, deposit_root);
	}

	/// Update withdraw root
	fn update_withdraw_root(
		withdraw_tree_id: T::TreeId,
		withdraw_root_index: T::RootIndex,
		withdraw_root: T::Element,
	) {
		// Update withdraw root
		CachedWithdrawRoots::<T, I>::insert(withdraw_tree_id, withdraw_root_index, withdraw_root);
	}

	/// TODO: keep track of last (30) deposit roots
	/// Mapping from vanchor id to array of deposit roots and withdraw roots
	/// ResourceId (webb rs) -> where proposal is stored, what data to execute etc.

	/// Just need map of resource id to edge metadata
	/// Similar to linkable tree but without chain id etc.

	/// Claim AP tokens and update rewards VAnchor
	/// Which resourceId the deposit happened on
	/// DepositRoot array lookup
	/// Which resourceId the withdraw happened on
	/// Same w/ withdraw
	/// Only one deposit/withdraw root no src/dest
	/// Should not be calling update_deposit_root etc when claiming
	/// That should be done by relayer or proposal
	/// Edges don't need to be updated - VAnchor exists only on here
	/// Commitment into VAnchor for how much AP owed
	/// No AP actually transferred, just a commitment
	/// When withdraw from APVanchor, minted AP tokens
	pub fn claim_ap(
		deposit_resource_id: ResourceId,
		withdraw_resource_id: ResourceId,
		recipient: T::AccountId,
		amount: BalanceOf<T, I>,
		merkle_root: T::Element,
		latest_leaf_index: T::LeafIndex,
		proof_data: ProofData<T::Element>,
		deposit_root: T::Element,
		withdraw_root: T::Element,
	) -> DispatchResultWithPostInfo {
		// Handle proof verification
		Self::handle_proof_verification(&proof_data)?;

		let deposit_tree_id: T::TreeId = match deposit_resource_id.target_system() {
			TargetSystem::Substrate(system) => system.tree_id.into(),
			_ => {
				ensure!(false, Error::<T, I>::InvalidResourceId);
				T::TreeId::default()
			},
		};

		let withdraw_tree_id: T::TreeId = match withdraw_resource_id.target_system() {
			TargetSystem::Substrate(system) => system.tree_id.into(),
			_ => {
				ensure!(false, Error::<T, I>::InvalidResourceId);
				T::TreeId::default()
			},
		};

		let deposit_root_index = Self::next_deposit_root_index();
		let withdraw_root_index = Self::next_withdraw_root_index();

		NextDepositRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::DepositRootHistorySize::get()
		});
		NextWithdrawRootIndex::<T, I>::mutate(|i| {
			*i = i.saturating_add(One::one()) % T::DepositRootHistorySize::get()
		});

		// Updating roots, TODO: modify
		Self::update_deposit_root(deposit_tree_id, deposit_root_index, deposit_root);
		Self::update_withdraw_root(withdraw_tree_id, withdraw_root_index, withdraw_root);

		// Flag deposit nullifier as being used, TODO: modify
		for nullifier in &proof_data.input_nullifiers {
			Self::add_deposit_nullifier_hash(deposit_tree_id, *nullifier)?;
		}

		// Insert output commitments into the AP tree
		for comm in &proof_data.output_commitments {
			T::LinkableTree::insert_in_order(deposit_tree_id, *comm)?;
		}

		Ok(().into())
	}

	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}
}
