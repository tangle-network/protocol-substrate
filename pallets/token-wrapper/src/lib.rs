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
//! * Wrapping assets
//! * Unwrapping share pooled tokens
//!
//! To use it in your runtime, you need to implement the hasher [`Config`].
//! Additionally, you will want to implement the hash traits defined in the
//! darkwebb_primitives::hasher module.
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

// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// mod tests;

mod traits;

use codec::{Decode, Encode};
use sp_std::{prelude::*, vec};

use asset_registry::{Registry, ShareTokenRegistry};
use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::traits::AccountIdConversion,
	traits::{Currency, Get},
	PalletId,
};
use orml_traits::MultiCurrency;
use traits::TokenWrapperInterface;

pub use pallet::*;

/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> =
	<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
pub type NativeBalance<T, I> =
	<<T as Config<I>>::Balances as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + asset_registry::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency mechanism.
		type Currency: MultiCurrency<Self::AccountId>;

		/// Default Currency implementation
		type Balances: Currency<Self::AccountId>;

		/// Asset registry
		type AssetRegistry: Registry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>
			+ ShareTokenRegistry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>;
	}

	#[pallet::storage]
	#[pallet::getter(fn wrapping_fee)]
	/// Percentage of amount to used as wrapping fee
	pub type WrappingFee<T: Config<I>, I: 'static = ()> = StorageValue<_, NativeBalance<T, I>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		WrappedToken {
			pool_share_asset: T::AssetId,
			asset_id: T::AssetId,
			amount: T::Balance,
			recipient: T::AccountId,
		},
		UnwrappedToken {
			pool_share_asset: T::AssetId,
			asset_id: T::AssetId,
			amount: T::Balance,
			recipient: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Invalid transaction amount
		InvalidAmount,
		/// Poolshare asset not found
		InvalidPoolShareAsset,
		/// AssetId not found in selected pool share
		UnregisteredAssetId,
		/// Assets not found in selected pool
		NotFoundInPool,
		/// Insufficient Balance
		InsufficientBalance,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn wrap(
			origin: OriginFor<T>,
			from_asset_id: T::AssetId,
			into_pool_share_id: T::AssetId,
			amount: T::Balance,
			recipient: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			<Self as TokenWrapperInterface<T::AccountId, T::AssetId, T::Balance>>::wrap(
				from_asset_id,
				into_pool_share_id,
				amount,
				recipient,
			)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn unwrap(
			origin: OriginFor<T>,
			from_pool_share_id: T::AssetId,
			into_asset_id: T::AssetId,
			amount: T::Balance,
			recipient: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			<Self as TokenWrapperInterface<T::AccountId, T::AssetId, T::Balance>>::unwrap(
				from_pool_share_id,
				into_asset_id,
				amount,
				recipient,
			)?;
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn to_currency_id(asset_id: T::AssetId) -> Result<CurrencyIdOf<T, I>, &'static str> {
		let bytes = asset_id.encode();
		CurrencyIdOf::<T, I>::decode(&mut &bytes[..]).map_err(|_| "Error converting asset_id to currency id")
	}

	pub fn to_balance(balance: T::Balance) -> BalanceOf<T, I> {
		let bytes = balance.encode();
		BalanceOf::<T, I>::decode(&mut &bytes[..]).unwrap_or_default()
	}

	pub fn has_sufficient_balance(recipient: &T::AccountId) -> bool {
		let wrapping_fee = Self::wrapping_fee();
		<T::Balances as Currency<T::AccountId>>::free_balance(recipient) > wrapping_fee
	}
}

impl<T: Config<I>, I: 'static> TokenWrapperInterface<T::AccountId, T::AssetId, T::Balance> for Pallet<T, I> {
	fn wrap(
		from_asset_id: T::AssetId,
		into_pool_share_id: T::AssetId,
		amount: T::Balance,
		recipient: T::AccountId,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		ensure!(
			<T::AssetRegistry as Registry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::exists(from_asset_id),
			Error::<T, I>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::contains_asset(
				into_pool_share_id,
				from_asset_id
			),
			Error::<T, I>::NotFoundInPool
		);

		let from_currency_id = Self::to_currency_id(from_asset_id)?;
		let pool_share_currency_id = Self::to_currency_id(into_pool_share_id)?;
		let value = Self::to_balance(amount);

		let wrapping_fee = Self::wrapping_fee();

		ensure!(
			Self::has_sufficient_balance(&recipient),
			Error::<T, I>::InsufficientBalance
		);

		// TODO Transfer to treasury

		<T::Balances as Currency<T::AccountId>>::transfer(
			&recipient,
			&Self::account_id(),
			wrapping_fee,
			frame_support::traits::ExistenceRequirement::KeepAlive,
		)?;

		T::Currency::transfer(from_currency_id, &recipient, &Self::account_id(), value)?;

		T::Currency::deposit(pool_share_currency_id, &recipient, value)?;

		Self::deposit_event(Event::WrappedToken {
			pool_share_asset: into_pool_share_id,
			asset_id: from_asset_id,
			amount,
			recipient,
		});
		Ok(())
	}

	fn unwrap(
		from_pool_share_id: T::AssetId,
		into_asset_id: T::AssetId,
		amount: T::Balance,
		recipient: T::AccountId,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		ensure!(
			<T::AssetRegistry as Registry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::exists(into_asset_id),
			Error::<T, I>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<T::AssetId, Vec<u8>, T::Balance, DispatchError>>::contains_asset(
				from_pool_share_id,
				into_asset_id
			),
			Error::<T, I>::NotFoundInPool
		);

		let into_currency_id = Self::to_currency_id(into_asset_id)?;
		let pool_share_currency_id = Self::to_currency_id(from_pool_share_id)?;

		let value = Self::to_balance(amount);

		T::Currency::withdraw(pool_share_currency_id, &recipient, value)?;

		T::Currency::transfer(into_currency_id, &Self::account_id(), &recipient, value)?;

		Self::deposit_event(Event::UnwrappedToken {
			pool_share_asset: from_pool_share_id,
			asset_id: into_asset_id,
			amount,
			recipient,
		});
		Ok(())
	}
}
