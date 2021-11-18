//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct VAnchorMetadata<AccountId, AssetId> {
	/// Creator account
	pub creator: AccountId,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: AssetId,
}
