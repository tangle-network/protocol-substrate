// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;

use frame_support::{traits::OneSessionHandler, Parameter};

use sp_runtime::{
	generic::DigestItem,
	traits::{IsMember, Member},
	RuntimeAppPublic,
};
use sp_std::prelude::*;

use webb_primitives::{AuthorityIndex, ConsensusLog, ValidatorSet, WEBB_ENGINE_ID};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Authority identifier type
		type WebbId: Member + Parameter + RuntimeAppPublic + Default + MaybeSerializeDeserialize;
		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(origin, new_maintainer));
				Ok(().into())
			})
		}

		#[pallet::weight(0)]
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(Default::default(), T::AccountId::default()));
				Ok(().into())
			})
		}

		#[pallet::weight(0)]
		pub fn set_threshold(origin: OriginFor<T>, new_threshold: u32) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			ensure!(new_threshold <= Authorities::<T>::get().len() as u32, Error::<T>::InvalidThreshold);
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T>::InvalidPermissions);
			// set the new maintainer
			SignatureThreshold::<T>::try_mutate(|threshold| {
				*threshold = new_threshold.clone();
				Self::deposit_event(Event::ThresholdSet(new_threshold));
				Ok(().into())
			})
		}

		#[pallet::weight(0)]
		pub fn force_set_threshold(origin: OriginFor<T>, new_threshold: u32) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			ensure!(new_threshold <= Authorities::<T>::get().len() as u32, Error::<T>::InvalidThreshold);
			// set the new maintainer
			SignatureThreshold::<T>::try_mutate(|threshold| {
				*threshold = new_threshold.clone();
				Self::deposit_event(Event::ThresholdSet(new_threshold));
				Ok(().into())
			})
		}
	}

	/// The current signature threshold (i.e. the `t` in t-of-n)
	#[pallet::storage]
	#[pallet::getter(fn signature_threshold)]
	pub(super) type SignatureThreshold<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The current authorities set
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::WebbId>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> = StorageValue<_, webb_primitives::ValidatorSetId, ValueQuery>;

	/// Authorities set scheduled to be used with the next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> = StorageValue<_, Vec<T::WebbId>, ValueQuery>;

	/// The parameter maintainer who can change the parameters
	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	pub(super) type Maintainer<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		ThresholdSet(u32),
		MaintainerSet(T::AccountId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Invalid threshold
		InvalidThreshold,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub authorities: Vec<T::WebbId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				authorities: Vec::new(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_authorities(&self.authorities);
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Return the current active BEEFY validator set.
	pub fn validator_set() -> ValidatorSet<T::WebbId> {
		ValidatorSet::<T::WebbId> {
			validators: Self::authorities(),
			id: Self::validator_set_id(),
		}
	}

	fn change_authorities(new: Vec<T::WebbId>, queued: Vec<T::WebbId>) {
		// As in GRANDPA, we trigger a validator set change only if the the validator
		// set has actually changed.
		if new != Self::authorities() {
			<Authorities<T>>::put(&new);

			let next_id = Self::validator_set_id() + 1u64;
			<ValidatorSetId<T>>::put(next_id);

			let log: DigestItem<T::Hash> = DigestItem::Consensus(
				WEBB_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(ValidatorSet {
					validators: new,
					id: next_id,
				})
				.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(log);
		}

		<NextAuthorities<T>>::put(&queued);
	}

	fn initialize_authorities(authorities: &[T::WebbId]) {
		if authorities.is_empty() {
			return;
		}

		assert!(
			<Authorities<T>>::get().is_empty(),
			"Authorities are already initialized!"
		);

		<Authorities<T>>::put(authorities);
		<ValidatorSetId<T>>::put(0);
		// Like `pallet_session`, initialize the next validator set as well.
		<NextAuthorities<T>>::put(authorities);
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::WebbId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::WebbId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::WebbId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		Self::initialize_authorities(&authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::WebbId)>,
	{
		if changed {
			let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
			let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();

			Self::change_authorities(next_authorities, next_queued_authorities);
		}
	}

	fn on_disabled(i: usize) {
		let log: DigestItem<T::Hash> = DigestItem::Consensus(
			WEBB_ENGINE_ID,
			ConsensusLog::<T::WebbId>::OnDisabled(i as AuthorityIndex).encode(),
		);

		<frame_system::Pallet<T>>::deposit_log(log);
	}
}

impl<T: Config> IsMember<T::WebbId> for Pallet<T> {
	fn is_member(authority_id: &T::WebbId) -> bool {
		Self::authorities().iter().any(|id| id == authority_id)
	}
}
