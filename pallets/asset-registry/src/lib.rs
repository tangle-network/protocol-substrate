// This file is part of Basilisk-node.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchError, pallet_prelude::*, sp_runtime::traits::CheckedAdd, transactional,
};
use frame_system::pallet_prelude::*;
use sp_arithmetic::traits::BaseArithmetic;
use sp_std::{convert::TryInto, vec::Vec};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod benchmarking;
mod traits;
mod types;
pub mod weights;
use std::convert::TryFrom;

use weights::WeightInfo;

pub use types::AssetType;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

use crate::types::{AssetDetails, AssetMetadata};
use frame_support::BoundedVec;
pub use traits::{Registry, ShareTokenRegistry};
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::fmt::Debug, sp_runtime::traits::AtLeast32BitUnsigned};

	pub type AssetDetailsT<T> = AssetDetails<
		<T as Config>::AssetId,
		<T as Config>::Balance,
		BoundedVec<u8, <T as Config>::StringLimit>,
		<T as Config>::MaxAssetIdInPool,
	>;

	pub type AssetTypeOf<T> = AssetType<<T as Config>::AssetId, <T as Config>::MaxAssetIdInPool>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The origin which can work with asset-registry.
		type RegistryOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Asset type
		type AssetId: Parameter
			+ Member
			+ Default
			+ Copy
			+ BaseArithmetic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Balance type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Asset location type
		type AssetNativeLocation: Parameter + Member + Default + MaxEncodedLen;

		/// The maximum length of a name or symbol stored on-chain.
		type StringLimit: Get<u32>;

		/// The maximum number of assets in a pool
		type MaxAssetIdInPool: Get<u32> + Clone + Debug + TypeInfo + Eq + PartialEq;

		/// Native Asset Id
		#[pallet::constant]
		type NativeAssetId: Get<Self::AssetId>;

		/// Weight information for the extrinsics
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset Id is not available. This only happens when it reaches the MAX
		/// value of given id type.
		NoIdAvailable,

		/// Invalid asset name or symbol.
		AssetNotFound,

		/// Invalid asset name or symbol.
		TooLong,

		/// Asset ID is not registered in the asset-registry.
		AssetNotRegistered,

		/// Asset is already registered.
		AssetAlreadyRegistered,

		/// Incorrect number of assets provided to create shared asset.
		InvalidSharedAssetLen,

		/// Asset exists in to pool
		AssetExistsInPool,

		/// Asset not found in pool
		AssetNotFoundInPool,

		/// Max number of assets in pool is reached
		MaxAssetIdInPoolReached,
	}

	#[pallet::storage]
	#[pallet::getter(fn assets)]
	/// Details of an asset.
	pub type Assets<T: Config> =
		StorageMap<_, Twox64Concat, T::AssetId, AssetDetailsT<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_asset_id)]
	/// Next available asset id. This is sequential id assigned for each new
	/// registered asset.
	pub type NextAssetId<T: Config> = StorageValue<_, T::AssetId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn asset_ids)]
	/// Mapping between asset name and asset id.
	pub type AssetIds<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, T::StringLimit>, T::AssetId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locations)]
	/// Native location of an asset.
	pub type AssetLocations<T: Config> =
		StorageMap<_, Twox64Concat, T::AssetId, T::AssetNativeLocation, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn location_assets)]
	/// Local asset for native location.
	pub type LocationAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AssetNativeLocation, T::AssetId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn asset_metadata)]
	/// Metadata of an asset.
	pub type AssetMetadataMap<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AssetId,
		AssetMetadata<BoundedVec<u8, T::StringLimit>>,
		OptionQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub asset_names: Vec<(BoundedVec<u8, T::StringLimit>, T::Balance)>,
		pub native_asset_name: BoundedVec<u8, T::StringLimit>,
		pub native_existential_deposit: T::Balance,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig::<T> {
				asset_names: vec![],
				native_asset_name: b"WEBB".to_vec().try_into().unwrap(),
				native_existential_deposit: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Register native asset first
			// It is to make sure that native is registered as any other asset
			let native_asset_name = Pallet::<T>::to_bounded_name(self.native_asset_name.to_vec())
				.map_err(|_| panic!("Invalid native asset name!"))
				.unwrap();

			AssetIds::<T>::insert(&native_asset_name, T::NativeAssetId::get());
			let details = AssetDetails {
				name: native_asset_name,
				asset_type: AssetType::Token,
				existential_deposit: self.native_existential_deposit,
				locked: false,
			};

			Assets::<T>::insert(T::NativeAssetId::get(), details);

			self.asset_names.iter().for_each(|(name, ed)| {
				let bounded_name = Pallet::<T>::to_bounded_name(name.to_vec())
					.map_err(|_| panic!("Invalid asset name!"))
					.unwrap();
				let _ = Pallet::<T>::register_asset(bounded_name, AssetType::Token, *ed)
					.map_err(|_| panic!("Failed to register asset"));
			})
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset was registered.
		Registered {
			asset_id: T::AssetId,
			name: BoundedVec<u8, T::StringLimit>,
			asset_type: AssetTypeOf<T>,
		},

		/// Asset was updated.
		Updated {
			asset_id: T::AssetId,
			name: BoundedVec<u8, T::StringLimit>,
			asset_type: AssetTypeOf<T>,
		},

		/// Metadata set for an asset.
		MetadataSet { asset_id: T::AssetId, symbol: BoundedVec<u8, T::StringLimit>, decimals: u8 },

		/// Native location set for an asset.
		LocationSet { asset_id: T::AssetId, location: T::AssetNativeLocation },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new asset.
		///
		/// Asset is identified by `name` and the name must not be used to
		/// register another asset.
		///
		/// New asset is given `NextAssetId` - sequential asset id
		///
		/// Adds mapping between `name` and assigned `asset_id` so asset id can
		/// be retrieved by name too (Note: this approach is used in AMM
		/// implementation (xyk))
		///
		/// Emits 'Registered` event when successful.
		#[pallet::weight(<T as Config>::WeightInfo::register())]
		#[transactional]
		pub fn register(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::StringLimit>,
			asset_type: AssetTypeOf<T>,
			existential_deposit: T::Balance,
		) -> DispatchResult {
			T::RegistryOrigin::ensure_origin(origin)?;

			ensure!(Self::asset_ids(&name).is_none(), Error::<T>::AssetAlreadyRegistered);

			Self::register_asset(name, asset_type, existential_deposit)?;

			Ok(())
		}

		/// Update registered asset.
		///
		/// Updates also mapping between name and asset id if provided name is
		/// different than currently registered.
		///
		/// Emits `Updated` event when successful.

		// TODO: No tests
		#[pallet::weight(<T as Config>::WeightInfo::update())]
		#[transactional]
		pub fn update(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			bounded_name: BoundedVec<u8, T::StringLimit>,
			asset_type: AssetTypeOf<T>,
			existential_deposit: Option<T::Balance>,
		) -> DispatchResult {
			T::RegistryOrigin::ensure_origin(origin)?;

			Assets::<T>::try_mutate(asset_id, |maybe_detail| -> DispatchResult {
				let mut detail = maybe_detail.as_mut().ok_or(Error::<T>::AssetNotFound)?;

				if bounded_name != detail.name {
					// Make sure that there is no such name already registered
					ensure!(
						Self::asset_ids(&bounded_name).is_none(),
						Error::<T>::AssetAlreadyRegistered
					);

					// update also name map - remove old one first
					AssetIds::<T>::remove(&detail.name);
					AssetIds::<T>::insert(&bounded_name, asset_id);
				}

				detail.name = bounded_name.clone();
				detail.asset_type = asset_type.clone();
				detail.existential_deposit =
					existential_deposit.unwrap_or(detail.existential_deposit);

				Self::deposit_event(Event::Updated { asset_id, name: bounded_name, asset_type });

				Ok(())
			})
		}

		/// Set metadata for an asset.
		///
		/// - `asset_id`: Asset identifier.
		/// - `symbol`: The exchange symbol for this asset. Limited in length by `StringLimit`.
		/// - `decimals`: The number of decimals this asset uses to represent one unit.
		///
		/// Emits `MetadataSet` event when successful.
		#[pallet::weight(<T as Config>::WeightInfo::set_metadata())]
		#[transactional]
		pub fn set_metadata(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			symbol: BoundedVec<u8, T::StringLimit>,
			decimals: u8,
		) -> DispatchResult {
			T::RegistryOrigin::ensure_origin(origin)?;

			ensure!(Self::assets(asset_id).is_some(), Error::<T>::AssetNotFound);

			let metadata = AssetMetadata::<BoundedVec<u8, T::StringLimit>> {
				symbol: symbol.clone(),
				decimals,
			};

			AssetMetadataMap::<T>::insert(asset_id, metadata);

			Self::deposit_event(Event::MetadataSet { asset_id, symbol, decimals });

			Ok(())
		}

		/// Set asset native location.
		///
		/// Adds mapping between native location and local asset id and vice
		/// versa.
		///
		/// Mainly used in XCM.
		///
		/// Emits `LocationSet` event when successful.
		#[pallet::weight(<T as Config>::WeightInfo::set_location())]
		#[transactional]
		pub fn set_location(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			location: T::AssetNativeLocation,
		) -> DispatchResult {
			T::RegistryOrigin::ensure_origin(origin)?;

			ensure!(Self::assets(asset_id).is_some(), Error::<T>::AssetNotRegistered);

			AssetLocations::<T>::insert(asset_id, &location);
			LocationAssets::<T>::insert(&location, asset_id);

			Self::deposit_event(Event::LocationSet { asset_id, location });

			Ok(())
		}

		/// Add an asset to an existing pool.
		#[pallet::weight(0)]
		pub fn add_asset_to_pool(
			origin: OriginFor<T>,
			pool: BoundedVec<u8, T::StringLimit>,
			asset_id: T::AssetId,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(Self::assets(asset_id).is_some(), Error::<T>::AssetNotRegistered);

			Self::add_asset_to_existing_pool(&pool, asset_id)?;

			Ok(())
		}

		/// Remove an asset from an existing pool.
		#[pallet::weight(0)]
		pub fn delete_asset_from_pool(
			origin: OriginFor<T>,
			pool: BoundedVec<u8, T::StringLimit>,
			asset_id: T::AssetId,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(Self::assets(asset_id).is_some(), Error::<T>::AssetNotRegistered);

			Self::delete_asset_from_existing_pool(&pool, asset_id)?;

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Convert BoundedVec<u8, T::StringLimit> to BoundedVec so it respects the max set limit,
	/// otherwise return TooLong error
	fn to_bounded_name(name: Vec<u8>) -> Result<BoundedVec<u8, T::StringLimit>, Error<T>> {
		name.try_into().map_err(|_| Error::<T>::TooLong)
	}

	/// Register new asset.
	///
	/// Does not perform any  check whether an asset for given name already
	/// exists. This has to be prior to calling this function.
	pub fn register_asset(
		name: BoundedVec<u8, T::StringLimit>,
		asset_type: AssetTypeOf<T>,
		existential_deposit: T::Balance,
	) -> Result<T::AssetId, DispatchError> {
		NextAssetId::<T>::mutate(|value| -> Result<T::AssetId, DispatchError> {
			// Check if current id does not clash with CORE ASSET ID.
			// If yes, just skip it and use next one, otherwise use it.
			// Note: this way we prevent accidental clashes with native asset id, so no need
			// to set next asset id to be > next asset id
			let asset_id = if *value == T::NativeAssetId::get() {
				value.checked_add(&T::AssetId::from(1)).ok_or(Error::<T>::NoIdAvailable)?
			} else {
				*value
			};

			// Map name to asset id
			AssetIds::<T>::insert(&name, asset_id);

			let details = AssetDetails {
				name: name.clone(),
				asset_type: asset_type.clone(),
				existential_deposit,
				locked: false,
			};

			// Store the details
			Assets::<T>::insert(asset_id, details);

			// Increase asset id to be assigned for following asset.
			*value = asset_id.checked_add(&T::AssetId::from(1)).ok_or(Error::<T>::NoIdAvailable)?;

			Self::deposit_event(Event::Registered { asset_id, name, asset_type });

			Ok(asset_id)
		})
	}

	/// Create asset for given name or return existing AssetId if such asset
	/// already exists.
	pub fn get_or_create_asset(
		name: BoundedVec<u8, T::StringLimit>,
		asset_type: AssetTypeOf<T>,
		existential_deposit: T::Balance,
	) -> Result<T::AssetId, DispatchError> {
		if let Some(asset_id) = AssetIds::<T>::get(&name) {
			Ok(asset_id)
		} else {
			Self::register_asset(name, asset_type, existential_deposit)
		}
	}

	/// Return location for given asset.
	pub fn asset_to_location(asset_id: T::AssetId) -> Option<T::AssetNativeLocation> {
		Self::locations(asset_id)
	}

	/// Return asset for given loation.
	pub fn location_to_asset(location: T::AssetNativeLocation) -> Option<T::AssetId> {
		Self::location_assets(location)
	}

	#[allow(clippy::ptr_arg)]
	pub fn add_asset_to_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
	) -> Result<T::AssetId, DispatchError> {
		let pool_asset_id = Self::retrieve_asset(name)?;
		Assets::<T>::try_mutate(
			pool_asset_id,
			move |maybe_detail| -> Result<T::AssetId, DispatchError> {
				let detail = maybe_detail.as_mut().ok_or(Error::<T>::AssetNotFound)?;

				let asset_type = match &detail.asset_type {
					AssetType::Token => return Err(Error::<T>::AssetNotFound.into()),
					AssetType::PoolShare(pool) =>
						if !pool.contains(&asset_id) {
							if Self::assets(asset_id).is_some() {
								let mut pool_clone = pool.clone();
								pool_clone
									.try_push(asset_id)
									.map_err(|_| Error::<T>::MaxAssetIdInPoolReached)?;
								AssetType::PoolShare(pool_clone)
							} else {
								return Err(Error::<T>::AssetNotRegistered.into())
							}
						} else {
							return Err(Error::<T>::AssetExistsInPool.into())
						},
				};

				detail.asset_type = asset_type.clone();

				Self::deposit_event(Event::Updated {
					asset_id: pool_asset_id,
					name: detail.name.clone(),
					asset_type,
				});

				Ok(pool_asset_id)
			},
		)
	}

	#[allow(clippy::ptr_arg)]
	pub fn delete_asset_from_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
	) -> Result<T::AssetId, DispatchError> {
		let pool_asset_id = Self::retrieve_asset(name)?;
		Assets::<T>::try_mutate(
			pool_asset_id,
			move |maybe_detail| -> Result<T::AssetId, DispatchError> {
				let detail = maybe_detail.as_mut().ok_or(Error::<T>::AssetNotFound)?;

				let asset_type = match &detail.asset_type {
					AssetType::Token => return Err(Error::<T>::AssetNotFound.into()),
					AssetType::PoolShare(pool) =>
						if pool.contains(&asset_id) {
							let filtered_pool = pool
								.iter()
								.filter(|id| **id != asset_id)
								.copied()
								.collect::<Vec<T::AssetId>>();
							let bounded_pool =
								BoundedVec::<T::AssetId, T::MaxAssetIdInPool>::try_from(
									filtered_pool,
								)
								.map_err(|_| Error::<T>::MaxAssetIdInPoolReached)?;
							AssetType::PoolShare(bounded_pool)
						} else {
							return Err(Error::<T>::AssetNotFoundInPool.into())
						},
				};

				detail.asset_type = asset_type.clone();

				Self::deposit_event(Event::Updated {
					asset_id: pool_asset_id,
					name: detail.name.clone(),
					asset_type,
				});

				Ok(pool_asset_id)
			},
		)
	}

	pub fn contains_asset(pool_share_id: T::AssetId, asset_id: T::AssetId) -> bool {
		<Self as ShareTokenRegistry<
			T::AssetId,
			Vec<u8>,
			T::Balance,
			BoundedVec<u8, T::StringLimit>,
			<T as Config>::MaxAssetIdInPool,
			DispatchError,
		>>::contains_asset(pool_share_id, asset_id)
	}
}

impl<T: Config>
	Registry<
		T::AssetId,
		Vec<u8>,
		T::Balance,
		BoundedVec<u8, T::StringLimit>,
		<T as Config>::MaxAssetIdInPool,
		DispatchError,
	> for Pallet<T>
{
	fn get_by_id(asset_id: T::AssetId) -> Result<AssetDetailsT<T>, DispatchError> {
		match Assets::<T>::get(asset_id) {
			Some(asset_details) => Ok(asset_details),
			None => Err(Error::<T>::AssetNotFound.into()),
		}
	}

	fn exists(asset_id: T::AssetId) -> bool {
		Assets::<T>::contains_key(asset_id) || asset_id == T::NativeAssetId::get()
	}

	fn retrieve_asset(name: &Vec<u8>) -> Result<T::AssetId, DispatchError> {
		let bounded_name = Self::to_bounded_name(name.clone())?;
		if let Some(asset_id) = AssetIds::<T>::get(bounded_name) {
			Ok(asset_id)
		} else {
			Err(Error::<T>::AssetNotFound.into())
		}
	}

	fn create_asset(
		name: &Vec<u8>,
		existential_deposit: T::Balance,
	) -> Result<T::AssetId, DispatchError> {
		let bounded_name = Self::to_bounded_name(name.clone())?;
		Self::get_or_create_asset(bounded_name.clone(), AssetType::Token, existential_deposit)
	}
}

impl<T: Config>
	ShareTokenRegistry<
		T::AssetId,
		Vec<u8>,
		T::Balance,
		BoundedVec<u8, T::StringLimit>,
		<T as Config>::MaxAssetIdInPool,
		DispatchError,
	> for Pallet<T>
{
	fn retrieve_shared_asset(
		name: &Vec<u8>,
		_assets: &[T::AssetId],
	) -> Result<T::AssetId, DispatchError> {
		Self::retrieve_asset(name)
	}

	fn create_shared_asset(
		name: &Vec<u8>,
		assets: &[T::AssetId],
		existential_deposit: T::Balance,
	) -> Result<T::AssetId, DispatchError> {
		let bounded_name = Self::to_bounded_name(name.clone())?;
		let bounded_pool = BoundedVec::<T::AssetId, T::MaxAssetIdInPool>::try_from(assets.to_vec())
			.map_err(|_| Error::<T>::MaxAssetIdInPoolReached)?;
		Self::get_or_create_asset(
			bounded_name,
			AssetType::PoolShare(bounded_pool),
			existential_deposit,
		)
	}

	fn contains_asset(pool_share_id: T::AssetId, asset_id: T::AssetId) -> bool {
		let pool_option = Self::assets(pool_share_id);
		if let Some(pool) = pool_option {
			match pool.asset_type {
				AssetType::Token => return false,
				AssetType::PoolShare(assets) => return assets.contains(&asset_id),
			}
		}

		false
	}

	fn add_asset_to_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
	) -> Result<T::AssetId, DispatchError> {
		Self::add_asset_to_existing_pool(name, asset_id)
	}

	fn delete_asset_from_existing_pool(
		name: &Vec<u8>,
		asset_id: T::AssetId,
	) -> Result<T::AssetId, DispatchError> {
		Self::delete_asset_from_existing_pool(name, asset_id)
	}
}

use orml_traits::GetByKey;

// Return Existential deposit of an asset
impl<T: Config> GetByKey<T::AssetId, T::Balance> for Pallet<T> {
	fn get(k: &T::AssetId) -> T::Balance {
		if let Some(details) = Self::assets(k) {
			details.existential_deposit
		} else {
			// Asset does not exists, so it does not really matter.
			Default::default()
		}
	}
}
