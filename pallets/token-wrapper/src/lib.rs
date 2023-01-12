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
use sp_std::prelude::*;

use asset_registry::{Registry, ShareTokenRegistry};
use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::traits::AccountIdConversion,
	traits::Get,
	BoundedVec, PalletId,
};
use orml_traits::MultiCurrency;
use sp_arithmetic::traits::Saturating;
use sp_runtime::traits::AtLeast32Bit;
use sp_std::convert::{TryFrom, TryInto};
use traits::TokenWrapperInterface;
use weights::WeightInfo;

pub use pallet::*;

/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T> =
	<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
pub type BalanceOf<T> =
	<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency mechanism.
		type Currency: MultiCurrency<Self::AccountId>;

		#[pallet::constant]
		type TreasuryId: Get<PalletId>;

		/// Asset registry
		type AssetRegistry: Registry<
				Self::AssetId,
				Vec<u8>,
				Self::Balance,
				BoundedVec<u8, Self::StringLimit>,
				Self::MaxAssetIdInPool,
				DispatchError,
			> + ShareTokenRegistry<
				Self::AssetId,
				Vec<u8>,
				Self::Balance,
				BoundedVec<u8, Self::StringLimit>,
				Self::MaxAssetIdInPool,
				DispatchError,
			>;

		/// Proposal nonce type
		type ProposalNonce: Encode
			+ Decode
			+ Parameter
			+ AtLeast32Bit
			+ Default
			+ Copy
			+ MaxEncodedLen;

		#[pallet::constant]
		type WrappingFeeDivider: Get<BalanceOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn wrapping_fee_percent)]
	/// Percentage of amount to be used as wrapping fee
	pub type WrappingFeePercent<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AssetId, BalanceOf<T>, OptionQuery>;

	/// Fee recipient, account which will be receiving wrapping cost fee.
	#[pallet::storage]
	#[pallet::getter(fn fee_recipient)]
	pub type FeeRecipient<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, T::StringLimit>, T::AccountId>;

	/// The proposal nonce used to prevent replay attacks on execute_proposal
	#[pallet::storage]
	#[pallet::getter(fn proposal_nonce)]
	pub type ProposalNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, T::StringLimit>, T::ProposalNonce>;

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
			into_pool_share_id: T::AssetId,
			wrapping_fee_percent: BalanceOf<T>,
		},
		UpdatedFeeRecipient {
			fee_recipient: T::AccountId,
			pool_share_id: T::AssetId,
		},
		TokensRescued {
			from_pool_share_id: T::AssetId,
			asset_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
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
		// No wrapping fee percentage found for the pool share
		NoWrappingFeePercentFound,
		/// Invalid nonce
		InvalidNonce,
		/// Name exceeds maximum limit
		NameExceedsMaximumLimit,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::set_wrapping_fee())]
		pub fn set_wrapping_fee(
			origin: OriginFor<T>,
			fee: BalanceOf<T>,
			into_pool_share_id: T::AssetId,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<Self as TokenWrapperInterface<
				T::AccountId,
				T::AssetId,
				BalanceOf<T>,
				T::ProposalNonce,
			>>::set_wrapping_fee(into_pool_share_id, fee, nonce)?;
			Ok(().into())
		}

		#[pallet::weight(195_000_000)]
		pub fn set_fee_recipient(
			origin: OriginFor<T>,
			pool_share_id: T::AssetId,
			fee_recipient: T::AccountId,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<Self as TokenWrapperInterface<
				T::AccountId,
				T::AssetId,
				BalanceOf<T>,
				T::ProposalNonce,
			>>::set_fee_recipient(pool_share_id, fee_recipient, nonce)?;
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

			<Self as TokenWrapperInterface<
				T::AccountId,
				T::AssetId,
				BalanceOf<T>,
				T::ProposalNonce,
			>>::wrap(origin, from_asset_id, into_pool_share_id, amount, recipient)?;
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

			<Self as TokenWrapperInterface<
				T::AccountId,
				T::AssetId,
				BalanceOf<T>,
				T::ProposalNonce,
			>>::unwrap(origin, from_pool_share_id, into_asset_id, amount, recipient)?;
			Ok(().into())
		}

		#[pallet::weight(195_000_000)]
		pub fn rescue_tokens(
			origin: OriginFor<T>,
			from_pool_share_id: T::AssetId,
			asset_id: T::AssetId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
			nonce: T::ProposalNonce,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<Self as TokenWrapperInterface<
				T::AccountId,
				T::AssetId,
				BalanceOf<T>,
				T::ProposalNonce,
			>>::rescue_tokens(from_pool_share_id, asset_id, amount, recipient, nonce)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub fn treasury_id() -> T::AccountId {
		T::TreasuryId::get().into_account_truncating()
	}

	pub fn get_fee_recipient(name: &BoundedVec<u8, T::StringLimit>) -> T::AccountId {
		FeeRecipient::<T>::get(name).unwrap_or(Self::treasury_id())
	}

	pub fn to_currency_id(asset_id: T::AssetId) -> Result<CurrencyIdOf<T>, &'static str> {
		let bytes = asset_id.encode();
		CurrencyIdOf::<T>::decode(&mut &bytes[..])
			.map_err(|_| "Error converting asset_id to currency id")
	}

	/// If X is amount of pooled share tokens and fee is 5%,
	/// need to wrap X / 0.95 total, so wrapping fee is
	/// (X / 0.95) - X
	pub fn get_wrapping_fee(
		amount: BalanceOf<T>,
		into_pool_share_id: T::AssetId,
	) -> Result<BalanceOf<T>, DispatchError> {
		let percent = WrappingFeePercent::<T>::get(into_pool_share_id).unwrap_or_default();
		Ok(amount.saturating_mul(percent) / T::WrappingFeeDivider::get().saturating_sub(percent))
	}

	pub fn get_amount_to_wrap(
		amount: BalanceOf<T>,
		into_pool_share_id: T::AssetId,
	) -> BalanceOf<T> {
		amount
			.saturating_add(Self::get_wrapping_fee(amount, into_pool_share_id).unwrap_or_default())
	}

	pub fn has_sufficient_balance(
		currency_id: CurrencyIdOf<T>,
		sender: &T::AccountId,
		amount: BalanceOf<T>,
		into_pool_share_id: T::AssetId,
	) -> bool {
		let total = Self::get_amount_to_wrap(amount, into_pool_share_id);
		T::Currency::free_balance(currency_id, sender) > total
	}

	pub fn get_balance(currency_id: CurrencyIdOf<T>, who: &T::AccountId) -> BalanceOf<T> {
		T::Currency::total_balance(currency_id, who)
	}

	pub fn validate_and_set_nonce(
		name: &BoundedVec<u8, T::StringLimit>,
		nonce: T::ProposalNonce,
	) -> Result<(), DispatchError> {
		// Nonce should be greater than the proposal nonce in storage
		let proposal_nonce = ProposalNonce::<T>::get(name).unwrap_or_default();
		// let proposal_nonce = ProposalNonce::<T>::get();

		ensure!(proposal_nonce < nonce, Error::<T>::InvalidNonce);

		// Nonce should increment by a maximum of 1,048
		ensure!(
			nonce <= proposal_nonce + T::ProposalNonce::from(1_048u32),
			Error::<T>::InvalidNonce
		);
		// Set the new nonce
		ProposalNonce::<T>::insert(name, nonce);
		Ok(())
	}
}

impl<T: Config> TokenWrapperInterface<T::AccountId, T::AssetId, BalanceOf<T>, T::ProposalNonce>
	for Pallet<T>
{
	fn wrap(
		from: T::AccountId,
		from_asset_id: T::AssetId,
		into_pool_share_id: T::AssetId,
		amount: BalanceOf<T>,
		recipient: T::AccountId,
	) -> Result<(), DispatchError> {
		ensure!(amount > <BalanceOf<T>>::default(), Error::<T>::InvalidAmount);

		ensure!(
			<T::AssetRegistry as Registry<
				T::AssetId,
				Vec<u8>,
				T::Balance,
				BoundedVec<u8, T::StringLimit>,
				T::MaxAssetIdInPool,
				DispatchError,
			>>::exists(from_asset_id),
			Error::<T>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<
				T::AssetId,
				Vec<u8>,
				T::Balance,
				BoundedVec<u8, T::StringLimit>,
				T::MaxAssetIdInPool,
				DispatchError,
			>>::contains_asset(into_pool_share_id, from_asset_id),
			Error::<T>::NotFoundInPool
		);

		let from_currency_id = Self::to_currency_id(from_asset_id)?;
		let pool_share_currency_id = Self::to_currency_id(into_pool_share_id)?;

		ensure!(
			Self::has_sufficient_balance(from_currency_id, &from, amount, into_pool_share_id),
			Error::<T>::InsufficientBalance
		);

		let asset_details = <T::AssetRegistry as Registry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::get_by_id(into_pool_share_id)?;

		T::Currency::transfer(
			from_currency_id,
			&from,
			&Self::get_fee_recipient(&asset_details.name),
			Self::get_wrapping_fee(amount, into_pool_share_id)?,
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
			<T::AssetRegistry as Registry<
				T::AssetId,
				Vec<u8>,
				T::Balance,
				BoundedVec<u8, T::StringLimit>,
				T::MaxAssetIdInPool,
				DispatchError,
			>>::exists(into_asset_id),
			Error::<T>::UnregisteredAssetId
		);

		ensure!(
			<T::AssetRegistry as ShareTokenRegistry<
				T::AssetId,
				Vec<u8>,
				T::Balance,
				BoundedVec<u8, T::StringLimit>,
				T::MaxAssetIdInPool,
				DispatchError,
			>>::contains_asset(from_pool_share_id, into_asset_id),
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

	fn set_wrapping_fee(
		into_pool_share_id: T::AssetId,
		fee: BalanceOf<T>,
		nonce: T::ProposalNonce,
	) -> Result<(), DispatchError> {
		let asset_details = <T::AssetRegistry as Registry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::get_by_id(into_pool_share_id)?;
		// Nonce should be greater than the proposal nonce in storage
		Self::validate_and_set_nonce(&asset_details.name, nonce)?;

		WrappingFeePercent::<T>::insert(into_pool_share_id, fee);

		Self::deposit_event(Event::UpdatedWrappingFeePercent {
			wrapping_fee_percent: fee,
			into_pool_share_id,
		});
		Ok(())
	}
	// sets new fee recipient who will receiving wrapping cost fee.
	fn set_fee_recipient(
		pool_share_id: T::AssetId,
		fee_recipient: T::AccountId,
		nonce: T::ProposalNonce,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		let asset_details = <T::AssetRegistry as Registry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::get_by_id(pool_share_id)?;
		// nonce should be greater than the proposal nonce in storage
		Self::validate_and_set_nonce(&asset_details.name, nonce)?;
		// update fee recipient
		FeeRecipient::<T>::insert(asset_details.name, &fee_recipient);

		Self::deposit_event(Event::UpdatedFeeRecipient { fee_recipient, pool_share_id });
		Ok(())
	}

	// transfers tokens from treasury (fee recipient) to provided address.
	fn rescue_tokens(
		from_pool_share_id: T::AssetId,
		asset_id: T::AssetId,
		amount: BalanceOf<T>,
		recipient: T::AccountId,
		nonce: T::ProposalNonce,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		let asset_details = <T::AssetRegistry as Registry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::get_by_id(from_pool_share_id)?;
		// Nonce should be greater than the proposal nonce in storage
		Self::validate_and_set_nonce(&asset_details.name, nonce)?;
		// One which receives wrapping cost fees for provided asset
		let fee_recipient = Self::get_fee_recipient(&asset_details.name);
		let from_currency_id = Self::to_currency_id(asset_id)?;

		// Check if total balance of fee recipient id greater than amount to rescue
		let total_balance = T::Currency::total_balance(from_currency_id, &fee_recipient);
		if total_balance > amount {
			T::Currency::transfer(from_currency_id, &fee_recipient, &recipient, amount)?;
		} else {
			T::Currency::transfer(from_currency_id, &fee_recipient, &recipient, total_balance)?;
		}

		Self::deposit_event(Event::TokensRescued {
			from_pool_share_id,
			asset_id,
			amount,
			recipient,
		});
		Ok(())
	}

	fn add_asset_to_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
		nonce: T::ProposalNonce,
	) -> Result<T::AssetId, DispatchError> {
		// Nonce should be greater than the proposal nonce in storage
		let bounded_name = BoundedVec::<u8, T::StringLimit>::try_from(name.clone())
			.map_err(|_e| Error::<T>::NameExceedsMaximumLimit)?;
		Self::validate_and_set_nonce(&bounded_name, nonce)?;
		<T::AssetRegistry as ShareTokenRegistry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::add_asset_to_existing_pool(name, asset_id)
	}

	fn delete_asset_from_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
		nonce: T::ProposalNonce,
	) -> Result<T::AssetId, DispatchError> {
		// Nonce should be greater than the proposal nonce in storage
		let bounded_name = BoundedVec::<u8, T::StringLimit>::try_from(name.clone())
			.map_err(|_e| Error::<T>::NameExceedsMaximumLimit)?;
		Self::validate_and_set_nonce(&bounded_name, nonce)?;
		<T::AssetRegistry as ShareTokenRegistry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			T::MaxAssetIdInPool,
			DispatchError,
		>>::delete_asset_from_existing_pool(name, asset_id)
	}
}
