//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, Debug, TypeInfo)]
pub struct EdgeMetadata<ChainIdWithType, Element, LastLeafIndex> {
	/// chain id with type
	pub src_id_with_type: ChainIdWithType,
	/// root of source chain anchor's native merkle tree
	pub root: Element,
	/// height of source chain anchor's native merkle tree
	pub latest_leaf_index: LastLeafIndex,
}
