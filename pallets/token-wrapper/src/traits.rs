use frame_support::dispatch;

pub trait TokenWrapperInterface<AccountId, AssetId, Balance> {
	fn wrap(
		fromAssetId: AssetId,
		intoPoolShareId: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
	fn unwrap(
		fromPoolShareId: AssetId,
		intoAssetId: AssetId,
		amount: Balance,
		recipient: AccountId,
	) -> Result<(), dispatch::DispatchError>;
}
