use frame_support::{dispatch::fmt::Debug, pallet_prelude::*, BoundedVec};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetType<AssetId, MaxAssetIdInPool: Get<u32> + Clone + Debug + Eq + PartialEq> {
	Token,
	PoolShare(BoundedVec<AssetId, MaxAssetIdInPool>),
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetDetails<
	AssetId,
	Balance,
	BoundedString,
	MaxAssetIdInPool: Get<u32> + Clone + Debug + Eq + PartialEq,
> {
	/// The name of this asset. Limited in length by `StringLimit`.
	pub name: BoundedString,

	pub asset_type: AssetType<AssetId, MaxAssetIdInPool>,

	pub existential_deposit: Balance,

	pub locked: bool,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AssetMetadata<BoundedString> {
	/// The ticker symbol for this asset. Limited in length by `StringLimit`.
	pub symbol: BoundedString,
	/// The number of decimals this asset uses to represent one unit.
	pub decimals: u8,
}
