use crate::*;
use frame_support::dispatch::fmt::Debug;

pub trait Registry<
	AssetId,
	AssetName,
	Balance,
	BoundedString,
	MaxAssetIdInPool: Get<u32> + Clone + Debug + Eq + PartialEq,
	Error,
>
{
	fn get_by_id(
		id: AssetId,
	) -> Result<AssetDetails<AssetId, Balance, BoundedString, MaxAssetIdInPool>, Error>;

	fn exists(name: AssetId) -> bool;

	fn retrieve_asset(name: &AssetName) -> Result<AssetId, Error>;

	fn create_asset(name: &AssetName, existential_deposit: Balance) -> Result<AssetId, Error>;

	fn get_or_create_asset(
		name: AssetName,
		existential_deposit: Balance,
	) -> Result<AssetId, Error> {
		if let Ok(asset_id) = Self::retrieve_asset(&name) {
			Ok(asset_id)
		} else {
			Self::create_asset(&name, existential_deposit)
		}
	}
}

pub trait ShareTokenRegistry<
	AssetId,
	AssetName,
	Balance,
	BoundedString,
	MaxAssetIdInPool: Get<u32> + Clone + Debug + Eq + PartialEq,
	Error,
>: Registry<AssetId, AssetName, Balance, BoundedString, MaxAssetIdInPool, Error>
{
	fn retrieve_shared_asset(name: &AssetName, assets: &[AssetId]) -> Result<AssetId, Error>;

	fn create_shared_asset(
		name: &AssetName,
		assets: &[AssetId],
		existential_deposit: Balance,
	) -> Result<AssetId, Error>;

	fn get_or_create_shared_asset(
		name: AssetName,
		assets: Vec<AssetId>,
		existential_deposit: Balance,
	) -> Result<AssetId, Error> {
		if let Ok(asset_id) = Self::retrieve_shared_asset(&name, &assets) {
			Ok(asset_id)
		} else {
			Self::create_shared_asset(&name, &assets, existential_deposit)
		}
	}

	fn contains_asset(pool_share_id: AssetId, asset_id: AssetId) -> bool;
	fn add_asset_to_existing_pool(name: &Vec<u8>, asset_id: AssetId) -> Result<AssetId, Error>;
	fn delete_asset_from_existing_pool(name: &Vec<u8>, asset_id: AssetId)
		-> Result<AssetId, Error>;
}
