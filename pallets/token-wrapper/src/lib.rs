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

//! # Token Wrapper Module
//!
//! A module for wrapping pooled assets and minting pool share tokens
//!
//! ## Overview
//!
//! The Token Wrapper module provides functionality for token wrapping
//! management including:
//!
//! * Wrapping assets into shares pool tokens
//! * Unwrapping shared pool tokens
//!
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The token wrapper in Webb is designed to make the following possible:
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

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod traits;
pub mod weights;

use codec::{Decode, Encode};
use sp_runtime::traits::Saturating;
use sp_std::prelude::*;

use asset_registry::{Registry, ShareTokenRegistry};
use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::traits::AccountIdConversion,
	traits::Get,
	PalletId,
};
use orml_traits::MultiCurrency;
use traits::TokenWrapperInterface;
use weights::WeightInfo;

pub use pallet::*;

/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T> =
	<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
pub type BalanceOf<T> = <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config + asset_registry::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency mechanism.
		type Currency: MultiCurrency<Self::AccountId>;

		#[pallet::constant]
		type TreasuryId: Get<PalletId>;

		/// Asset registry
		type AssetRegistry: Registry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>
			+ ShareTokenRegistry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>;

		#[pallet::constant]
		type WrappingFeeDivider: Get<BalanceOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn wrapping_fee_percent)]
	/// Percentage of amount to be used as wrapping fee
	pub type WrappingFeePercent<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		WrappedToken {
			pool_share_asset: T::AssetId,
			asset_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
		},
		UnwrappedToken {
			pool_share_asset: T::AssetId,
			asset_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
		},
		UpdatedWrappingFeePercent {
			wrapping_fee_percent: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid transaction amount
		InvalidAmount,
		/// AssetId not found in selected pool share
		UnregisteredAssetId,
		/// Assets not found in selected pool
		NotFoundInPool,
		/// Insufficient Balance for an asset
		InsufficientBalance,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::set_wrapping_fee())]
		pub fn set_wrapping_fee(origin: OriginFor<T>, fee: BalanceOf<T>) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<Self as TokenWrapperInterface<T::AccountId, T::AssetId, BalanceOf<T>>>::set_wrapping_fee(fee)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::wrap())]
		pub fn wrap(
			origin: OriginFor<T>,
			from_asset_id: T::AssetId,
			into_pool_share_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			<Self as TokenWrapperInterface<T::AccountId, T::AssetId, BalanceOf<T>>>::wrap(
				origin,
				from_asset_id,
				into_pool_share_id,
				amount,
				recipient,
			)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::unwrap())]
		pub fn unwrap(
			origin: OriginFor<T>,
			from_pool_share_id: T::AssetId,
			into_asset_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			<Self as TokenWrapperInterface<T::AccountId, T::AssetId, BalanceOf<T>>>::unwrap(
				origin,
				from_pool_share_id,
				into_asset_id,
				amount,
				recipient,
			)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn treasury_id() -> T::AccountId {
		T::TreasuryId::get().into_account()
	}

	pub fn to_currency_id(asset_id: T::AssetId) -> Result<CurrencyIdOf<T>, &'static str> {
		let bytes = asset_id.encode();
		CurrencyIdOf::<T>::decode(&mut &bytes[..]).map_err(|_| "Error converting asset_id to currency id")
	}

	/// If X is amount of pooled share tokens and fee is 5%,
	/// need to wrap X / 0.95 total, so wrapping fee is
	/// (X / 0.95) - X
	pub fn get_wrapping_fee(amount: BalanceOf<T>) -> BalanceOf<T> {
		let percent = Self::wrapping_fee_percent();
		amount.saturating_mul(percent) / T::WrappingFeeDivider::get().saturating_sub(percent)
	}

	pub fn get_amount_to_wrap(amount: BalanceOf<T>) -> BalanceOf<T> {
		amount.saturating_add(Self::get_wrapping_fee(amount))
	}

	pub fn has_sufficient_balance(currency_id: CurrencyIdOf<T>, sender: &T::AccountId, amount: BalanceOf<T>) -> bool {
		let total = Self::get_amount_to_wrap(amount);
		T::Currency::free_balance(currency_id, sender) > total
	}

	pub fn get_balance(currency_id: CurrencyIdOf<T>, who: &T::AccountId) -> BalanceOf<T> {
		T::Currency::total_balance(currency_id, who)
	}
}

impl<T: Config> TokenWrapperInterface<T::AccountId, T::AssetId, BalanceOf<T>> for Pallet<T> {
	fn set_wrapping_fee(fee: BalanceOf<T>) -> Result<(), DispatchError> {
		WrappingFeePercent::<T>::put(fee);

		Self::deposit_event(Event::UpdatedWrappingFeePercent {
			wrapping_fee_percent: fee,
		});
		Ok(().into())
	}

	fn wrap(
		from: T::AccountId,
		from_asset_id: T::AssetId,
		into_pool_share_id: T::AssetId,
		amount: BalanceOf<T>,
		recipient: T::AccountId,
	) -> Result<(), DispatchError> {
		ensure!(amount > <BalanceOf<T>>::default(), Error::<T>::InvalidAmount);

		ensure!(
			<T::AssetRegistry as Registry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::exists(from_asset_id),
			Error::<T>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::contains_asset(
				into_pool_share_id,
				from_asset_id
			),
			Error::<T>::NotFoundInPool
		);

		let from_currency_id = Self::to_currency_id(from_asset_id)?;
		let pool_share_currency_id = Self::to_currency_id(into_pool_share_id)?;

		ensure!(
			Self::has_sufficient_balance(from_currency_id, &from, amount),
			Error::<T>::InsufficientBalance
		);

		T::Currency::transfer(
			from_currency_id,
			&from,
			&Self::treasury_id(),
			Self::get_wrapping_fee(amount),
		)?;

		T::Currency::transfer(from_currency_id, &from, &Self::account_id(), amount)?;

		T::Currency::deposit(pool_share_currency_id, &recipient, amount)?;

		Self::deposit_event(Event::WrappedToken {
			pool_share_asset: into_pool_share_id,
			asset_id: from_asset_id,
			amount,
			recipient,
		});
		Ok(())
	}

	fn unwrap(
		from: T::AccountId,
		from_pool_share_id: T::AssetId,
		into_asset_id: T::AssetId,
		amount: BalanceOf<T>,
		recipient: T::AccountId,
	) -> Result<(), DispatchError> {
		ensure!(amount > <BalanceOf<T>>::default(), Error::<T>::InvalidAmount);

		ensure!(
			<T::AssetRegistry as Registry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::exists(into_asset_id),
			Error::<T>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::contains_asset(
				from_pool_share_id,
				into_asset_id
			),
			Error::<T>::NotFoundInPool
		);

		let into_currency_id = Self::to_currency_id(into_asset_id)?;
		let pool_share_currency_id = Self::to_currency_id(from_pool_share_id)?;

		T::Currency::withdraw(pool_share_currency_id, &from, amount)?;

		T::Currency::transfer(into_currency_id, &Self::account_id(), &recipient, amount)?;

		Self::deposit_event(Event::UnwrappedToken {
			pool_share_asset: from_pool_share_id,
			asset_id: into_asset_id,
			amount,
			recipient,
		});
		Ok(())
	}

	fn add_asset_to_existing_pool(name: &Vec<u8>, asset_id: T::AssetId) -> Result<T::AssetId, DispatchError> {
		<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::add_asset_to_existing_pool(name, asset_id)
	}

	fn delete_asset_from_existing_pool(name: &Vec<u8>, asset_id: T::AssetId) -> Result<T::AssetId, DispatchError> {
		<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::delete_asset_from_existing_pool(name, asset_id)
	}
}
