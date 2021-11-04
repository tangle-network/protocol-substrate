//! All the traits exposed to be used in other custom pallets
use frame_support::dispatch;
use sp_std::vec::Vec;

pub trait AnchorConfig {
	type LeafIndex;
	type AccountId;
	type Balance;
	type CurrencyId;
	type ChainId;
	type TreeId;
	type Element;
}

/// Anchor trait definition to be used in other pallets
pub trait AnchorInterface<C: AnchorConfig> {
	// Creates a new anchor
	fn create(
		creator: C::AccountId,
		deposit_size: C::Balance,
		depth: u8,
		max_edges: u32,
		asset: C::CurrencyId,
	) -> Result<C::TreeId, dispatch::DispatchError>;
	/// Deposit into the anchor
	fn deposit(account: C::AccountId, id: C::TreeId, leaf: C::Element) -> Result<(), dispatch::DispatchError>;
	/// Withdraw from the anchor
	fn withdraw(
		id: C::TreeId,
		proof_bytes: &[u8],
		chain_id: C::ChainId,
		roots: Vec<C::Element>,
		nullifier_hash: C::Element,
		recipient: C::AccountId,
		relayer: C::AccountId,
		fee: C::Balance,
		refund: C::Balance,
	) -> Result<(), dispatch::DispatchError>;
	/// Add an edge to this anchor
	fn add_edge(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		root: C::Element,
		latest_leaf_index: C::LeafIndex,
	) -> Result<(), dispatch::DispatchError>;
	/// Update an edge for this anchor
	fn update_edge(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		root: C::Element,
		latest_leaf_index: C::LeafIndex,
	) -> Result<(), dispatch::DispatchError>;
	// Stores nullifier hash from a spend tx
	fn add_nullifier_hash(id: C::TreeId, nullifier_hash: C::Element) -> Result<(), dispatch::DispatchError>;
}

/// Anchor trait for inspecting tree state
pub trait AnchorInspector<C: AnchorConfig> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_root(id: C::TreeId) -> Result<C::Element, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_root(id: C::TreeId, target: C::Element) -> Result<bool, dispatch::DispatchError>;

	fn ensure_known_root(id: C::TreeId, target: C::Element) -> Result<(), dispatch::DispatchError>;

	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_neighbor_roots(id: C::TreeId) -> Result<Vec<C::Element>, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_neighbor_root(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		target: C::Element,
	) -> Result<bool, dispatch::DispatchError>;

	// let is_known = Self::is_known_neighbor_root(id, src_chain_id, target)?;
	// ensure!(is_known, Error::<T, I>::InvalidNeighborWithdrawRoot);
	// Ok(())
	fn ensure_known_neighbor_root(
		id: C::TreeId,
		src_chain_id: C::ChainId,
		target: C::Element,
	) -> Result<(), dispatch::DispatchError>;
	/// Check if this anchor has this edge
	fn has_edge(id: C::TreeId, src_chain_id: C::ChainId) -> bool;

	/// Check if a nullifier has been used in a tree or returns
	/// `InvalidNullifier`
	fn is_nullifier_used(id: C::TreeId, nullifier: C::Element) -> bool;

	fn ensure_nullifier_unused(id: C::TreeId, nullifier: C::Element) -> Result<(), dispatch::DispatchError>;
}
