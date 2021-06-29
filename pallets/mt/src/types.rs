//! All the traits exposed to be used in other custom pallets
use frame_support::Parameter;
use crate::*;
use frame_support::dispatch;
use codec::{Encode, Decode};

pub trait ElementTrait: Encode + Decode + Parameter + Default + Copy {
	/// converts type to byte slice
	fn to_bytes(&self) -> &[u8];
	/// converts type to Vec
	fn to_vec(&self) -> Vec<u8> {
		self.to_bytes().to_vec()
	}
	/// converts slice to type
	fn from_bytes(bytes: &[u8]) -> Self;
	/// converts Vec to type
	fn from_vec(vec: Vec<u8>) -> Self {
		Self::from_bytes(&vec)
	}
}

/// Tree trait definition to be used in other pallets
pub trait TreeInterface<T: Config<I>, I: 'static = ()> {
	// Creates a new tree
	fn create(creator: T::AccountId, depth: u8) -> Result<T::TreeId, dispatch::DispatchError>;
	/// Adds members/leaves to the tree
	fn insert(id: T::TreeId, leaf: T::Element, index: T::LeafIndex) -> Result<T::Element, dispatch::DispatchError>;
}

/// Tree trait for inspecting tree state
pub trait TreeInspector<T: Config<I>, I: 'static = ()> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_root(id: T::TreeId) -> Result<T::Element, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns `TreeDoesntExist
	fn is_known_root(id: T::TreeId, target: T::Element) -> Result<bool, dispatch::DispatchError>;
}

#[derive(Default, Clone, Encode, Decode)]
pub struct TreeMetadata<AccountId, LeafIndex, Element> {
	/// Creator account
	pub creator: AccountId,
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
	pub edge_nodes: Vec<Element>,
}