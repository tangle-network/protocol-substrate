use frame_support::dispatch;

pub trait TokenWrapperInterface<AccountId, AssetId, Balance> {
	fn wrap(
		from_asset_id: AssetId,
		into_pool_share_id: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
	fn unwrap(
		from_pool_share_id: AssetId,
		into_asset_id: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
}
