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

//! # MASP Asset Registry Handler Module
//!
//! A module for executing add/remove asset proposals to the list of available
//! assets on the masp. These functions can only be called by the bridge.
//!
//! ## Overview
//!
//! The Token Wrapper Handler module provides functionality for token wrapping
//! management including:
//!
//! * Executing a proposal to add a token to the asset registry
//! * Executing a proposal to remove a token from the asset registry
//! * Executing a proposal to add a NFT to the asset registry
//! * Executing a proposal to remove a NFT from the asset registry
//!
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Dispatchable Functions
//! * execute_add_wrapped_fungible_asset
//! * execute_remove_wrapped_fungible_asset
//! * execute_add_wrapped_nft_asset
//! * execute_remove_wrapped_nft_asset
//!
//! ## Interface
//!
//! ## Related Modules

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock_signature_bridge;
#[cfg(test)]
mod tests_signature_bridge;

mod traits;

use frame_support::traits::EnsureOrigin;
use frame_system::pallet_prelude::OriginFor;
use sp_std::{convert::TryInto, vec::Vec};


pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use traits::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use pallet_token_wrapper::BalanceOf;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_token_wrapper::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type BridgeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// MaspAssetRegistry Interface
		type MaspAssetRegistry: MaspAssetRegistry<Self::AssetId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	pub enum Event<T: Config> {
		UpdatedWrappingFeePercent { wrapping_fee_percent: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Access violation.
		InvalidPermissions,
		// Token Wrapper Handler already exists for specified resource Id.
		ResourceIsAlreadyAnchored,
		// Token Wrapper Handler doesn't exist for specified resource Id.
		TokenWrapperHandlerNotFound,
		/// Storage overflowed.
		StorageOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(0)]
		pub fn execute_add_wrapped_fungible_asset(
			origin: OriginFor<T>,
			token_handler: EvmAddress,
			name: Vec<u8>,
			asset_id: T::AssetId,
			symbol: [u8; 32],
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			// Add asset to the MASP
			T::MaspAssetRegistry::execute_add_wrapped_fungible_asset(token_handler, name, asset_id, symbol)?;
			Ok(().into())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(1)]
		pub fn execute_remove_token_from_pool_share(
			origin: OriginFor<T>,
			token_handler: EvmAddress,
			name: Vec<u8>,
			asset_id: T::AssetId,
			symbol: [u8; 32],
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			// Remove asset to the MASP
			T::MaspAssetRegistry::execute_remove_wrapped_fungible_asset(token_handler, name, asset_id, symbol)?;
			Ok(().into())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(2)]
		pub fn execute_add_wrapped_nft_asset(
			origin: OriginFor<T>,
			token_handler: EvmAddress,
			asset_id: T::AssetId,
			nft_collection_address: EvmAddress,
			salt: [u8; 32],
			uri: [u8; 64],
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			// Add asset to the MASP
			T::MaspAssetRegistry::execute_add_wrapped_nft_asset(token_handler, asset_id, nft_collection_address, salt, uri)?;
			Ok(().into())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(3)]
		pub fn execute_remove_wrapped_nft_asset(
			origin: OriginFor<T>,
			token_handler: EvmAddress,
			asset_id: T::AssetId,
			nft_collection_address: EvmAddress,
			salt: [u8; 32],
			uri: [u8; 64],
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			// Remove asset to the MASP
			T::MaspAssetRegistry::execute_remove_wrapped_nft_asset(token_handler, asset_id, nft_collection_address, salt, uri)?;
			Ok(().into())
		}
	}
}
