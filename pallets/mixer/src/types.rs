//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MixerMetadata<Balance, AssetId> {
	/// Balance size of deposit
	pub deposit_size: Balance,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: AssetId,
}
