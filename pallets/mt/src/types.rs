//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedVec;
use scale_info::TypeInfo;

#[derive(Default, Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct TreeMetadata<AccountId, LeafIndex, Element, MaxEdges: Get<u32>> {
	/// Creator account
	pub creator: Option<AccountId>,
	/// Is paused
	pub paused: bool,
	/// Current number of leaves in the tree
	pub leaf_count: LeafIndex,
	/// Maximum allowed leaves in the tree
	pub max_leaves: LeafIndex,
	/// Depth of the tree
	pub depth: u8,
	/// The root hash of the tree
	pub root: Element,
	/// Edge nodes of tree, used to compute roots on the fly
	pub edge_nodes: BoundedVec<Element, MaxEdges>,
}
