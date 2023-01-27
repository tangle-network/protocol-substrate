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

//! # Anonymity Mining Module
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

		/// The overarching leaf index type
		// type RootIndex: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy +
		// MaxEncodedLen;

		/// Account Identifier from which the internal Pot is generated.
		type PotId: Get<PalletId>;

		// /// Pallet id
		// #[pallet::constant]
		// type PalletId: Get<PalletId>;

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

		// /// Time provider
		type Time: Time;

		/// Start time
		#[pallet::constant]
		type StartTimestamp: Get<u64>;

		#[pallet::constant]
		type Duration: Get<u64>;

		#[pallet::constant]
		type InitialLiquidity: Get<u64>;

		#[pallet::constant]
		type Liquidity: Get<u64>;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// TODO: keep track of last (30) deposit roots
	/// Mapping from vanchor id to array of deposit roots and withdraw roots
	/// ResourceId (webb rs) -> where proposal is stored, what data to execute etc.

	/// Just need map of resource id to edge metadata
	/// Similar to linkable tree but without chain id etc.

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

	// /// Map of resourceId to array of withdraw roots
	// #[pallet::storage]
	// #[pallet::getter(fn withdraw_roots)]
	// pub type WithdrawRoots<T: Config<I>, I: 'static = ()> =
	// 	StorageMap<_, Blake2_128Concat, T::TreeId, Vec<T::Element>, ValueQuery>;

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
		pub fn swap(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			amount: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			let tokens = Self::get_expected_return(&Self::account_id(), amount);

			let tokens_sold_u64 = tokens.saturated_into::<u64>();
			let prev_tokens_sold_u64 = Self::get_tokens_sold();
			Self::set_tokens_sold(prev_tokens_sold_u64 + tokens_sold_u64);

			// Deposit AP tokens to the pallet
			<T as Config<I>>::Currency::transfer(
				T::AnonymityPointsAssetId::get(),
				&recipient,
				&Self::account_id(),
				amount,
			)?;

			// Pallet sends reward tokens
			<T as Config<I>>::Currency::transfer(
				T::RewardAssetId::get(),
				&Self::account_id(),
				&recipient,
				tokens,
			)?;

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

	pub fn handle_proof_verification(
		proof_data: &ProofData<T::Element>,
	) -> Result<(), DispatchError> {
		// Stubbed out for now
		Ok(())
	}

	/// Update vanchor
	fn update_vanchor(
		tree_id: T::TreeId,
		merkle_root: T::Element,
		src_resource_id: ResourceId,
		latest_leaf_index: T::LeafIndex,
	) -> DispatchResultWithPostInfo {
		let src_chain_id: T::ChainId = src_resource_id.typed_chain_id().chain_id().into();
		if T::VAnchor::has_edge(tree_id, src_chain_id) {
			T::VAnchor::update_edge(
				tree_id,
				src_chain_id,
				merkle_root,
				latest_leaf_index,
				src_resource_id,
			)?;
			Self::deposit_event(Event::AnchorEdgeUpdated);
		} else {
			T::VAnchor::add_edge(
				tree_id,
				src_chain_id,
				merkle_root,
				latest_leaf_index,
				src_resource_id,
			)?;
			Self::deposit_event(Event::AnchorEdgeAdded);
		}
		Ok(().into())
	}

	fn update_roots(
		src_tree_id: T::TreeId,
		dest_tree_id: T::TreeId,
		src_root_index: T::RootIndex,
		dest_root_index: T::RootIndex,
		src_deposit_root: T::Element,
		src_withdraw_root: T::Element,
		dest_deposit_root: T::Element,
		dest_withdraw_root: T::Element,
	) {
		// Update roots
		CachedDepositRoots::<T, I>::insert(src_tree_id, src_root_index, src_deposit_root);
		CachedWithdrawRoots::<T, I>::insert(src_tree_id, src_root_index, src_withdraw_root);
		CachedDepositRoots::<T, I>::insert(dest_tree_id, dest_root_index, dest_deposit_root);
		CachedWithdrawRoots::<T, I>::insert(dest_tree_id, dest_root_index, dest_withdraw_root);
	}

	/// Claim AP tokens and update rewards VAnchor
	pub fn claim_ap(
		src_resource_id: ResourceId,
		dest_resource_id: ResourceId,
		recipient: T::AccountId,
		amount: BalanceOf<T, I>,
		merkle_root: T::Element,
		latest_leaf_index: T::LeafIndex,
		proof_data: ProofData<T::Element>,
		src_deposit_root: T::Element,
		src_withdraw_root: T::Element,
		dest_deposit_root: T::Element,
		dest_withdraw_root: T::Element,
	) -> DispatchResultWithPostInfo {
		// Transfer AP tokens to recipient
		<T as Config<I>>::Currency::transfer(
			T::AnonymityPointsAssetId::get(),
			&Self::account_id(),
			&recipient,
			amount,
		)?;

		let source_tree_id: T::TreeId = match src_resource_id.target_system() {
			TargetSystem::Substrate(system) => system.tree_id.into(),
			_ => {
				ensure!(false, Error::<T, I>::InvalidResourceId);
				T::TreeId::default()
			},
		};

		let dest_tree_id: T::TreeId = match dest_resource_id.target_system() {
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

		// Update roots - TODO: modify
		Self::update_roots(
			source_tree_id,
			dest_tree_id,
			deposit_root_index,
			withdraw_root_index,
			src_deposit_root,
			src_withdraw_root,
			dest_deposit_root,
			dest_withdraw_root,
		);

		// Update rewards VAnchor
		let vanchor =
			Self::update_vanchor(source_tree_id, merkle_root, src_resource_id, latest_leaf_index)?;

		// Handle proof verification
		Self::handle_proof_verification(&proof_data)?;

		// Flag deposit nullifier as being used, TODO: modify
		for nullifier in &proof_data.input_nullifiers {
			Self::add_deposit_nullifier_hash(source_tree_id, *nullifier)?;
		}

		// Insert output commitments into the AP tree
		for comm in &proof_data.output_commitments {
			T::LinkableTree::insert_in_order(source_tree_id, *comm)?;
		}

		// TODO: event?

		Ok(().into())
	}

	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}

	// Set pool weight
	pub fn set_pool_weight(new_pool_weight: u64) -> Result<(), DispatchError> {
		PoolWeight::<T, I>::set(new_pool_weight);
		Self::deposit_event(Event::UpdatedPoolWeight { pool_weight: new_pool_weight });
		Ok(())
	}

	// Set tokens sold
	pub fn set_tokens_sold(new_tokens_sold: u64) -> Result<(), DispatchError> {
		TokensSold::<T, I>::set(new_tokens_sold);
		Self::deposit_event(Event::UpdatedTokensSold { tokens_sold: new_tokens_sold });
		Ok(())
	}

	// Get current timestamp
	pub fn get_current_timestamp() -> u64 {
		let current_timestamp = T::Time::now();

		current_timestamp.saturated_into::<u64>()
	}

	/// Get expected number of tokens to swap
	pub fn get_expected_return(addr: &T::AccountId, amount: BalanceOf<T, I>) -> BalanceOf<T, I> {
		let old_balance = Self::get_virtual_balance(addr);
		let pool_weight = Self::get_pool_weight();
		let amount_u64: u64 = amount.saturated_into::<u64>();
		let pool_weight_u64: u64 = pool_weight.saturated_into::<u64>();
		let amount_i64: i64 = amount_u64 as i64;
		let pool_weight_i64: i64 = pool_weight_u64 as i64;
		let amount_fp: FixedI64 = FixedPointNumber::from_inner(amount_i64);
		let pool_weight_fp: FixedI64 = FixedPointNumber::from_inner(pool_weight_i64);
		let pow = -(amount_fp / pool_weight_fp);

		let pow_f64: f64 = pow.to_float();
		let exp = pow_f64.exp();

		let old_balance_u64 = old_balance.saturated_into::<u64>();

		let old_balance_f64 = old_balance_u64 as f64;
		let final_new_balance_f64 = old_balance_f64 * exp;
		let final_new_balance_i64 = final_new_balance_f64.round();
		let final_new_balance_u64 = final_new_balance_i64 as u64;

		let _final_balance_new =
			old_balance.saturating_sub(final_new_balance_u64.saturated_into::<BalanceOf<T, I>>());
		let final_balance_new_u64 = old_balance_u64 - final_new_balance_u64;

		final_balance_new_u64.saturated_into::<BalanceOf<T, I>>()
	}

	/// Calculate balance to use
	pub fn get_virtual_balance(addr: &T::AccountId) -> BalanceOf<T, I> {
		let reward_balance =
			<T as Config<I>>::Currency::total_balance(T::RewardAssetId::get(), addr);
		let start_timestamp = T::StartTimestamp::get();
		let current_timestamp = T::Time::now();
		let start_timestamp_u64 = start_timestamp.saturated_into::<u64>();
		let current_timestamp_u64 = current_timestamp.saturated_into::<u64>();
		let elapsed_u64 = current_timestamp_u64.saturating_sub(start_timestamp_u64);
		let liquidity_u64 = T::Liquidity::get().saturated_into::<u64>();
		let tokens_sold = Self::get_tokens_sold();
		if elapsed_u64 <= <T as Config<I>>::Duration::get() {
			let _liquidity = T::Liquidity::get();
			let duration = T::Duration::get();
			let amount =
				T::InitialLiquidity::get() + (liquidity_u64 * elapsed_u64) / duration - tokens_sold;
			let modified_reward_balance = amount.saturated_into::<BalanceOf<T, I>>();
			//let elapsed_balance = elapsed.saturated_into::<BalanceOf<T, I>>();
			let _elapsed_balance = (elapsed_u64).saturated_into::<BalanceOf<T, I>>();
			modified_reward_balance
		} else {
			reward_balance
		}
	}
}
