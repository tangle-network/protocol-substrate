use frame_support::dispatch;
use sp_std::vec::Vec;

pub trait TokenWrapperInterface<AccountId, AssetId, Balance> {
	fn set_wrapping_fee(fee: Balance) -> Result<(), dispatch::DispatchError>;
	fn wrap(
		from: AccountId,
		from_asset_id: AssetId,
		into_pool_share_id: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
	fn unwrap(
		from: AccountId,
		from_pool_share_id: AssetId,
		into_asset_id: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
	fn add_asset_to_existing_pool(
		name: &Vec<u8>,
		asset_id: AssetId,
	) -> Result<AssetId, dispatch::DispatchError>;
	fn delete_asset_from_existing_pool(
		name: &Vec<u8>,
		asset_id: AssetId,
	) -> Result<AssetId, dispatch::DispatchError>;
}
