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

//! # VAnchor Verifier Module
//!
//! A module for abstracting over many arbitrary zero-knowledge verifiers
//! for arbitrary zero-knowledge gadgets splitting on vanchor length. This
//! pallet should store verifying keys and any other verification specific
//! parameters for different backends that we support in Webb's ecosystem of
//! runtime modules.
//!
//! ## Overview
//!
//! The VAnchor Verifier module provides functionality for zero-knowledge verifier
//! management including:
//!
//! * Setting parameters for zero-knowledge verifiers
//! * Setting the maintainer of the parameters
//!
//! To use it in your runtime, you need to implement the verifier [`Config`].
//! Additionally, you will want to implement the verifier traits defined in the
//! webb_primitives::verifier module.
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

mod benchmarking;

use sp_std::convert::TryInto;
pub mod weights;
use sp_std::prelude::*;

use frame_support::pallet_prelude::{ensure, DispatchError};
use webb_primitives::verifier::*;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, pallet_prelude::*, Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]

	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The verifier instance trait
		type Verifier: InstanceVerifier;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The max parameter length accepted by the vanchor-verifier
		type MaxParameterLength: Get<u32>;

		/// WeightInfo for pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: (PhantomData<T>, PhantomData<I>),
		/// Optional vec of parameters (max edges, num_inputs, parameters)
		pub parameters: Option<Vec<(u8, u8, BoundedVec<u8, T::MaxParameterLength>)>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default(), parameters: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			if let Some(params) = &self.parameters {
				for p in params {
					Parameters::<T, I>::insert((p.0, p.1), p.2.clone());
				}
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn parameters)]
	/// Details of the module's parameters for different vanchor configurations
	pub(super) type Parameters<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		(u8, u8),
		BoundedVec<u8, T::MaxParameterLength>,
		ValueQuery,
	>;

	#[pallet::event]
	pub enum Event<T: Config<I>, I: 'static = ()> {}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Error during verification
		VerifyError,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::force_set_parameters(parameters.len() as u32))]
		pub fn force_set_parameters(
			origin: OriginFor<T>,
			configuration: (u8, u8),
			parameters: BoundedVec<u8, T::MaxParameterLength>,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			Parameters::<T, I>::try_mutate(configuration, |params| {
				*params = parameters.clone();
				Ok(().into())
			})
		}
	}
}

impl<T: Config<I>, I: 'static> VAnchorVerifierModule for Pallet<T, I> {
	fn verify(
		public_inp_bytes: &[u8],
		proof: &[u8],
		num_anchors: u8,
		num_inputs: u8,
	) -> Result<bool, DispatchError> {
		let params = Self::parameters((num_anchors, num_inputs));
		ensure!(!params.is_empty(), Error::<T, I>::ParametersNotInitialized);
		match T::Verifier::verify(public_inp_bytes, proof, &params) {
			Ok(verified) => Ok(verified),
			Err(e) => {
				log::error!("{:?}", e);
				ensure!(false, Error::<T, I>::VerifyError);
				Ok(false)
			},
		}
	}
}
