#![allow(clippy::ptr_arg, clippy::type_complexity)]
use frame_support::dispatch;
use sp_std::vec::Vec;

pub type EvmAddress = [u8; 20];

pub trait MaspAssetRegistry<AssetId> {
	fn execute_add_wrapped_fungible_asset(
		token_handler: EvmAddress,
		name: Vec<u8>,
		asset_id: AssetId,
		symbol: [u8; 32],
	) -> Result<(), dispatch::DispatchError>;
	fn execute_remove_wrapped_fungible_asset(
		token_handler: EvmAddress,
		name: Vec<u8>,
		asset_id: AssetId,
		symbol: [u8; 32],
	) -> Result<(), dispatch::DispatchError>;
	fn execute_add_wrapped_nft_asset(
		token_handler: EvmAddress,
		asset_id: AssetId,
		nft_collection_address: EvmAddress,
		salt: [u8; 32],
		uri: [u8; 64],
	) -> Result<(), dispatch::DispatchError>;
	fn execute_remove_wrapped_nft_asset(
		token_handler: EvmAddress,
		asset_id: AssetId,
		nft_collection_address: EvmAddress,
		salt: [u8; 32],
		uri: [u8; 64],
	) -> Result<(), dispatch::DispatchError>;
}
