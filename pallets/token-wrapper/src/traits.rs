use frame_support::dispatch;
use sp_std::vec::Vec;

pub trait TokenWrapperInterface<AccountId, AssetId, Balance, Nonce> {
	fn set_wrapping_fee(
		into_pool_share_id: AssetId,
		fee: Balance,
		nonce: Nonce,
	) -> Result<(), dispatch::DispatchError>;
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
		nonce: Nonce,
	) -> Result<AssetId, dispatch::DispatchError>;
	fn delete_asset_from_existing_pool(
		name: &Vec<u8>,
		asset_id: AssetId,
		nonce: Nonce,
	) -> Result<AssetId, dispatch::DispatchError>;
}
