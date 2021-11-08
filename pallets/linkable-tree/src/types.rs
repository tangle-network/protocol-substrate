//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, Debug, TypeInfo)]
pub struct EdgeMetadata<ChainID, Element, BlockNumber> {
	/// chain id
	pub src_chain_id: ChainID,
	/// root of source chain anchor's native merkle tree
	pub root: Element,
	/// height of source chain anchor's native merkle tree
	pub height: BlockNumber,
}
