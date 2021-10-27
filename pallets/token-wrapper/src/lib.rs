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
use sp_std::{prelude::*, vec};

use asset_registry::{Registry, ShareTokenRegistry};
use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::traits::AccountIdConversion,
	traits::Get,
	PalletId,
};
use orml_traits::MultiCurrency;
use sp_arithmetic::traits::BaseArithmetic;
use traits::TokenWrapperInterface;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, pallet_prelude::*, sp_runtime::traits::AtLeast32BitUnsigned,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency mechanism.
		type Currency: MultiCurrency<Self::AccountId>;

		/// Asset type
		type AssetId: Parameter + Member + Default + Copy + BaseArithmetic + MaybeSerializeDeserialize + MaxEncodedLen;

		/// Balance type
		type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize;

		/// Asset registry
		type AssetRegistry: Registry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>
			+ ShareTokenRegistry<Self::AssetId, Vec<u8>, Self::Balance, DispatchError>;
	}

	#[pallet::storage]
	#[pallet::getter(fn wrapping_fee_ratio)]
	/// Percentage of amount to used as wrapping fee
	pub type WrappingFeeRatio<T: Config<I>, I: 'static = ()> = StorageValue<_, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_supply)]
	/// Map of poolshare asset id to total supply
	pub type PoolShareAssetSupply<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, T::AssetId, T::Balance, ValueQuery>;

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
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn wrap(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn unwrap(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn mint(asset_id: T::AssetId, amount: T::Balance) {
		let mut total_supply = Self::total_supply(asset_id);

		total_supply += amount;

		PoolShareAssetSupply::<T, I>::insert(asset_id, total_supply);
	}

	pub fn burn(asset_id: T::AssetId, amount: T::Balance) {
		let total_supply = Self::total_supply(asset_id) - amount;

		PoolShareAssetSupply::<T, I>::insert(asset_id, total_supply);
	}
}

impl<T: Config<I>, I: 'static> TokenWrapperInterface<T::AccountId, T::AssetId, T::Balance> for Pallet<T, I> {
	fn wrap(
		fromAssetId: T::AssetId,
		intoPoolShareId: T::AssetId,
		amount: T::Balance,
		recipient: T::AccountId,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		todo!()
	}

	fn unwrap(
		fromPoolShareId: T::AssetId,
		intoAssetId: T::AssetId,
		amount: T::Balance,
		recipient: T::AccountId,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		todo!()
	}
}
