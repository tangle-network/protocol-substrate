//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct EdgeMetadata<ChainID, Element, LastLeafIndex> {
	/// chain id
	pub src_chain_id: ChainID,
	/// root of source chain anchor's native merkle tree
	pub root: Element,
	/// height of source chain anchor's native merkle tree
	pub latest_leaf_index: LastLeafIndex,
	/// Target contract address or tree identifier
	pub target: Element,
}
