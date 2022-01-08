use frame_support::dispatch;

pub trait TokenWrapperInterface<AccountId, AssetId, Balance> {
	fn set_wrapping_fee(

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
}
