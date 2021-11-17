//! All the traits exposed to be used in other custom pallets
use frame_support::dispatch;
use sp_std::vec::Vec;

pub trait VAnchorConfig {
	type LeafIndex;
	type AccountId;
	type Balance;
	type CurrencyId;
	type ChainId;
	type TreeId;
	type Element;
}

/// Anchor trait definition to be used in other pallets
pub trait VAnchorInterface<C: VAnchorConfig> {
	// Creates a new anchor
	fn create(
		creator: C::AccountId,
		depth: u8,
		max_edges: u32,
		asset: C::CurrencyId,
	) -> Result<C::TreeId, dispatch::DispatchError>;
	/// Transaction
	fn transact(
		id: C::TreeId,
		proof_bytes: &[u8],
		public_amount: C::Balance,
		ext_data: C::Element,
		input_nullifiers: Vec<C::Element>,
		output_commitments: Vec<C::Element>,
		roots: Vec<C::Element>,
	) -> Result<(), dispatch::DispatchError>;
	// Stores nullifier hash from a spend tx
	fn add_nullifier_hash(id: C::TreeId, nullifier_hash: C::Element) -> Result<(), dispatch::DispatchError>;
	/// Add an edge to this tree
	fn add_edge(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		root: C::Element,
		latest_leaf_index: C::LeafIndex,
	) -> Result<(), dispatch::DispatchError>;
	/// Update an edge for this tree
	fn update_edge(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		root: C::Element,
		latest_leaf_index: C::LeafIndex,
	) -> Result<(), dispatch::DispatchError>;
}

/// Anchor trait for inspecting tree state
pub trait VAnchorInspector<C: VAnchorConfig> {
	/// Check if a nullifier has been used in a tree or returns
	/// `InvalidNullifier`
	fn is_nullifier_used(id: C::TreeId, nullifier: C::Element) -> bool;
	/// Check if a nullifier has been used in a tree and throws if not
	fn ensure_nullifier_unused(id: C::TreeId, nullifier: C::Element) -> Result<(), dispatch::DispatchError>;
	/// Check if this linked tree has this edge (for backwards compatability)
	fn has_edge(id: C::TreeId, src_chain_id: C::ChainId) -> bool;
}
