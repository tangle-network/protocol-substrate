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

//! # Verifier Module
//!
//! A simple module for abstracting over arbitrary zero-knowledge verifiers
//! for arbitrary zero-knowledge gadgets. This pallet should store verifying
//! keys and any other verification specific parameters for different backends
//! that we support in Webb's ecosystem of runtime modules.
//!
//! ## Overview
//!
//! The Verifier module provides functionality for zero-knowledge verifier
//! management including:
//!
//! * Setting parameters for zero-knowledge verifier
//! * Setting the maintainer of the parameters
//!
//! To use it in your runtime, you need to implement the verifier [`Config`].
//! Additionally, you will want to implement the verifier traits defined in the
//! darkwebb_primitives::verifier module.
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The verifier system in Webb is designed to make the following possible:
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

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use sp_std::{prelude::*};
use sp_runtime::{
	traits::{
		Zero, Saturating,
	}
};

use frame_support::pallet_prelude::{ensure, DispatchError};
use frame_support::traits::{Currency, ReservableCurrency};
use frame_system::Config as SystemConfig;
use darkwebb_primitives::{types::DepositDetails, verifier::*};

type DepositBalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The verifier instance trait
		type Verifier: InstanceVerifier;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The origin which may forcibly reset parameters or otherwise alter privileged
		/// attributes.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		/// The basic amount of funds that must be reserved for an asset.
		type ParameterDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved when adding metadata to your parameters.
		type MetadataDepositBase: Get<DepositBalanceOf<Self, I>>;

		/// The additional funds that must be reserved for the number of bytes you store in your
		/// parameter metadata.
		type MetadataDepositPerByte: Get<DepositBalanceOf<Self, I>>;

		/// The maximum length of a name or symbol stored on-chain.
		type StringLimit: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn parameters)]
	/// Details of the module's parameters
	pub(super) type Parameters<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		Vec<u8>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn existing_deposit)]
	/// Details of the module's parameters
	pub(super) type Deposit<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		T::AccountId,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
	)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		ParametersSet(T::AccountId, Vec<u8>),
		MaintainerSet(T::AccountId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Parameters are invalid or empty
		InvalidParameters,
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Error during verification
		VerifyError,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn set_parameters(
			origin: OriginFor<T>,
			parameters: Vec<u8>
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// calculate the deposit
			let deposit = T::MetadataDepositPerByte::get()
				.saturating_mul((parameters.len() as u32).into())
				.saturating_add(T::MetadataDepositBase::get());
			// get old deposit details if they exist
			let old_deposit_details = Self::existing_deposit().unwrap_or(Default::default());
			// reserve and unreserve the currrency amounts
			if  old_deposit_details.depositor == origin {
				// handle when the current origin is the same as previous depositor
				if deposit > old_deposit_details.deposit {
					T::Currency::reserve(&origin, deposit - old_deposit_details.deposit)?;
				} else {
					T::Currency::unreserve(&origin, old_deposit_details.deposit - deposit);
				}
			} else {
				// handle when the current origin is different from old depositor
				T::Currency::reserve(&origin, deposit)?;
				T::Currency::unreserve(&old_deposit_details.depositor, old_deposit_details.deposit);
			}

			Parameters::<T, I>::try_mutate(|params| {
				*params = parameters.clone();
				Self::deposit_event(Event::ParametersSet(origin, parameters));
				Ok(().into())
			})
		}

		#[pallet::weight(0)]
		pub fn set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(origin, new_maintainer));
				Ok(().into())
			})

		}

		#[pallet::weight(0)]
		pub fn force_set_parameters(
			origin: OriginFor<T>,
			parameters: Vec<u8>
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// get old deposit details if they exist
			let old_deposit_details = Self::existing_deposit().unwrap_or(Default::default());
			// unreserve the currrency amounts from old depositor when force set
			if  old_deposit_details.deposit > DepositBalanceOf::<T, I>::zero() {
				T::Currency::unreserve(&old_deposit_details.depositor, old_deposit_details.deposit);
			}

			Deposit::<T, I>::kill();

			Parameters::<T, I>::try_mutate(|params| {
				*params = parameters.clone();
				Self::deposit_event(Event::ParametersSet(Default::default(), parameters));
				Ok(().into())
			})

		}

		#[pallet::weight(0)]
		pub fn force_set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(Default::default(), T::AccountId::default()));
				Ok(().into())
			})

		}
	}
}


impl<T: Config<I>, I: 'static> VerifierModule for Pallet<T, I> {
	fn verify(public_inp_bytes: &[u8], proof: &[u8]) -> Result<bool, DispatchError> {
		let params = Self::parameters();
		ensure!(params.len() != 0, Error::<T, I>::ParametersNotInitialized);
		match T::Verifier::verify(public_inp_bytes, proof, &params) {
			Ok(verified) => Ok(verified),
			Err(_) => {
				// TODO: Handle properly
				ensure!(false, Error::<T, I>::VerifyError);
				Ok(false)
			}
		}
	}
}