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
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::{
		traits::{AccountIdConversion, One, Saturating, Zero},
		FixedI64, FixedPointNumber, FixedPointOperand, SaturatedConversion,
	},
	traits::{Get, Time},
	PalletId,
};
use orml_traits::{currency::transactional, MultiCurrency};
use pallet_vanchor::VAnchorConfigration;
use sp_std::{convert::TryInto, prelude::*, vec};
use webb_primitives::{
	traits::vanchor::{VAnchorInspector, VAnchorInterface},
	types::{
		runtime::Moment,
		vanchor::{ExtData, ProofData, VAnchorMetadata},
	},
};

pub use pallet::*;

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
	#[pallet::without_storage_info]
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

		/// The tree
		type Tree: TreeInterface<Self::AccountId, Self::TreeId, Self::Element>
			+ TreeInspector<Self::AccountId, Self::TreeId, Self::Element>;

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
	#[pallet::getter(fn parameters)]
	/// Details of the module's parameters
	pub(super) type Parameters<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Vec<u8>, ValueQuery>;

	/// Miner
	#[pallet::storage]
	#[pallet::getter(fn get_reward_verifier)]
	pub type RewardVerifier<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_withdraw_verifier)]
	pub type RewardVerifier<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tree_update_verifier)]
	pub type TreeUpdateVerifier<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_governance)]
	pub type Governance<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tornado_trees)]
	pub type TornadoTrees<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	/// Rewards
	#[pallet::storage]
	#[pallet::getter(fn get_pool_weight)]
	pub type PoolWeight<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tokens_sold)]
	pub type TokensSold<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		VerifiersUpdated {
			sender: T::AccountId,
			reward_verifier: T::AccountId,
			withdraw_verifier: T::AccountId,
			tree_update_verifier: T::AccountId,
		},
		UpdatedPoolWeight {
			pool_weight: u64,
		},
		UpdatedTokensSold {
			tokens_sold: u64,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Error during hashing
		HashError,
		/// Invalid reward proof
		InvalidRewardProof,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
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

		// TODO: set_rates

		// Set tornado trees
		#[pallet::weight(0)]
		pub fn set_tornado_trees(
			origin: OriginFor<T>,
			tornado_trees: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::governance() == sender, "Only governance can perform this action");
			<TornadoTrees<T>>::put(tornado_trees);
			Ok(())
		}

		// Set verifiers
		#[pallet::weight(0)]
		pub fn set_verifiers(
			origin: OriginFor<T>,
			reward_verifier: T::AccountId,
			withdraw_verifier: T::AccountId,
			tree_update_verifier: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::governance() == sender, "Only governance can perform this action");

			<RewardVerifier<T>>::put(reward_verifier);
			<WithdrawVerifier<T>>::put(withdraw_verifier);
			<TreeUpdateVerifier<T>>::put(tree_update_verifier);

			Self::deposit_event(Event::VerifiersUpdated(
				sender,
				reward_verifier,
				withdraw_verifier,
				tree_update_verifier,
			));

			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}

	pub fn get_root(tree_id: T::TreeId) -> Result<T::Element, DispatchError> {
		T::Tree::get_root(tree_id)
	}

	pub fn is_known_root(
		tree_id: T::TreeId,
		target_root: T::Element,
	) -> Result<bool, DispatchError> {
		T::Tree::is_known_root(tree_id, target_root)
	}

	pub fn is_nullifier_used(tree_id: T::TreeId, nullifier_hash: T::Element) -> bool {
		NullifierHashes::<T, I>::contains_key(tree_id, nullifier_hash)
	}

	pub fn ensure_known_root(id: T::TreeId, target_root: T::Element) -> Result<(), DispatchError> {
		let is_known: bool = Self::is_known_root(id, target_root)?;
		ensure!(is_known, Error::<T, I>::UnknownRoot);
		Ok(())
	}

	pub fn ensure_nullifier_unused(
		id: T::TreeId,
		nullifier: T::Element,
	) -> Result<(), DispatchError> {
		ensure!(!Self::is_nullifier_used(id, nullifier), Error::<T, I>::AlreadyRevealedNullifier);
		Ok(())
	}

	/// Reward - TODO: modify
	fn reward(
		id: T::TreeId,
		proof_bytes: &[u8],
		root: T::Element,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		let mixer = Self::get_mixer(id)?;
		// Check if local root is known
		ensure!(T::Tree::is_known_root(id, root)?, Error::<T, I>::UnknownRoot);
		// Check nullifier and add or return `AlreadyRevealedNullifier`
		Self::ensure_nullifier_unused(id, nullifier_hash)?;
		Self::add_nullifier_hash(id, nullifier_hash)?;
		// Format proof public inputs for verification
		let mut bytes = Vec::new();
		let element_encoder = |v: &[u8]| {
			let mut output = [0u8; 32];
			output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
			output
		};
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);

		let mut arbitrary_data_bytes = Vec::new();
		arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
		arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
		arbitrary_data_bytes.extend_from_slice(&fee.encode());
		arbitrary_data_bytes.extend_from_slice(&refund.encode());
		let arbitrary_data = T::ArbitraryHasher::hash(&arbitrary_data_bytes, &[])
			.map_err(|_| Error::<T, I>::InvalidArbitraryData)?;

		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&root.encode());
		bytes.extend_from_slice(&arbitrary_data);
		let result = T::Verifier::verify(&bytes, proof_bytes)?;
		log::info!("verification result: {}", result);
		ensure!(result, Error::<T, I>::InvalidRewardProof);

		// TODO: check amount, if > fee then swap
		Ok(())
	}

	/// Withdraw - TODO: modify
	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		root: T::Element,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		let mixer = Self::get_mixer(id)?;
		// Check if local root is known
		ensure!(T::Tree::is_known_root(id, root)?, Error::<T, I>::UnknownRoot);
		// Check nullifier and add or return `AlreadyRevealedNullifier`
		Self::ensure_nullifier_unused(id, nullifier_hash)?;
		Self::add_nullifier_hash(id, nullifier_hash)?;
		// Format proof public inputs for verification
		let mut bytes = Vec::new();
		let element_encoder = |v: &[u8]| {
			let mut output = [0u8; 32];
			output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
			output
		};
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);

		let mut arbitrary_data_bytes = Vec::new();
		arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
		arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
		arbitrary_data_bytes.extend_from_slice(&fee.encode());
		arbitrary_data_bytes.extend_from_slice(&refund.encode());
		let arbitrary_data = T::ArbitraryHasher::hash(&arbitrary_data_bytes, &[])
			.map_err(|_| Error::<T, I>::InvalidArbitraryData)?;

		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&root.encode());
		bytes.extend_from_slice(&arbitrary_data);

		let result = T::Verifier::verify(&bytes, proof_bytes)?;
		log::info!("verification result: {}", result);
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);

		// TODO: check amount, if > fee then swap
		Ok(())
	}

	// Rewards

	// Set pool weight
	pub fn set_pool_weight(new_pool_weight: u64) -> Result<(), DispatchError> {
		PoolWeight::<T, I>::set(new_pool_weight);
		Self::deposit_event(Event::UpdatedPoolWeight { pool_weight: new_pool_weight });
		Ok(().into())
	}

	// Set tokens sold
	pub fn set_tokens_sold(new_tokens_sold: u64) -> Result<(), DispatchError> {
		TokensSold::<T, I>::set(new_tokens_sold);
		Self::deposit_event(Event::UpdatedTokensSold { tokens_sold: new_tokens_sold });
		Ok(().into())
	}

	// Get current timestamp
	pub fn get_current_timestamp() -> u64 {
		let current_timestamp = T::Time::now();
		let current_timestamp_u64 = current_timestamp.saturated_into::<u64>();
		return current_timestamp_u64
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

		let final_balance_new =
			old_balance.saturating_sub(final_new_balance_u64.saturated_into::<BalanceOf<T, I>>());
		let final_balance_new_u64 = old_balance_u64 - final_new_balance_u64;
		let final_balance = final_balance_new_u64.saturated_into::<BalanceOf<T, I>>();
		return final_balance
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
			let liquidity = T::Liquidity::get();
			let duration = T::Duration::get();
			let amount =
				T::InitialLiquidity::get() + (liquidity_u64 * elapsed_u64) / duration - tokens_sold;
			let modified_reward_balance = amount.saturated_into::<BalanceOf<T, I>>();
			//let elapsed_balance = elapsed.saturated_into::<BalanceOf<T, I>>();
			let elapsed_balance = (elapsed_u64).saturated_into::<BalanceOf<T, I>>();
			return modified_reward_balance
		} else {
			return reward_balance
		}
	}
}
