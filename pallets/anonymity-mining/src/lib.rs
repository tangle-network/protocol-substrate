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
		SaturatedConversion,
	},
	traits::{Get, Time},
	PalletId,
};
use orml_traits::{currency::transactional, MultiCurrency};
use pallet_vanchor::VAnchorConfigration;
use sp_std::{convert::TryInto, prelude::*, vec};
use webb_primitives::{
	traits::vanchor::{VAnchorInspector, VAnchorInterface},
	types::runtime::Moment,
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
		type PoolWeight: Get<u64>;

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

	#[pallet::event]
	pub enum Event<T: Config<I>, I: 'static = ()> {}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Error during hashing
		HashError,
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
			let tokens = Self::get_expected_return(&Self::account_id(), amount).unwrap();

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
	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}

	/// Get expected number of tokens to swap
	pub fn get_expected_return(
		addr: &T::AccountId,
		amount: BalanceOf<T, I>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		let old_balance = Self::get_virtual_balance(addr).unwrap();
		let pow =
			(amount.saturated_into::<u64>()).saturating_mul(<T as Config<I>>::PoolWeight::get());
		let e: u64 = 3;
		let zero: u64 = 0;
		//let neg_pow = zero.saturating_sub(pow);
		let exp = e.saturating_pow((pow / 100).try_into().unwrap());
		let new_balance = (old_balance.saturated_into::<u64>()).saturating_mul(exp) / 800;
		//let final_balance = old_balance - new_balance;
		let final_balance_new =
			old_balance.saturating_sub(new_balance.saturated_into::<BalanceOf<T, I>>());
		let final_balance = final_balance_new.saturated_into::<BalanceOf<T, I>>();
		Ok(final_balance)
	}

	/// Calculate balance to use
	pub fn get_virtual_balance(addr: &T::AccountId) -> Result<BalanceOf<T, I>, DispatchError> {
		let reward_balance =
			<T as Config<I>>::Currency::total_balance(T::RewardAssetId::get(), addr);
		let start_timestamp = T::StartTimestamp::get();
		let current_timestamp = T::Time::now();
		let elapsed = current_timestamp.saturated_into::<u64>() - start_timestamp;
		if elapsed <= <T as Config<I>>::Duration::get() {
			// TODO: initialLiquidity + (L * elapsed) / duration - tokensSold
			let liquidity = T::Liquidity::get();
			let duration = T::Duration::get();
			let amount = T::InitialLiquidity::get() +
				(liquidity.saturated_into::<u64>() * elapsed) / duration;
			let modified_reward_balance = amount.saturated_into::<BalanceOf<T, I>>();
			let elapsed_balance = elapsed.saturated_into::<BalanceOf<T, I>>();
			return Ok(reward_balance)
		} else {
			return Ok(reward_balance)
		}
	}
}
