//! All the traits exposed to be used in other custom pallets
use frame_support::dispatch;
use sp_std::vec::Vec;

/// Anchor trait definition to be used in other pallets
pub trait AnchorInterface<BlockNumber, AccountId, Balance, CurrencyId, ChainId, TreeId, Element> {
	// Creates a new anchor
	fn create(
		creator: AccountId,
		depth: u8,
		max_edges: u32,
		asset: CurrencyId,
	) -> Result<TreeId, dispatch::DispatchError>;
	/// Deposit into the anchor
	fn deposit(account: AccountId, id: TreeId, leaf: Element) -> Result<(), dispatch::DispatchError>;
	/// Withdraw from the anchor
	fn withdraw(
		id: TreeId,
		proof_bytes: &[u8],
		chain_id: ChainId,
		roots: Vec<Element>,
		nullifier_hash: Element,
		recipient: AccountId,
		relayer: AccountId,
		fee: Balance,
		refund: Balance,
	) -> Result<(), dispatch::DispatchError>;
	/// Add an edge to this anchor
	fn add_edge(
		id: TreeId,
		src_chain_id: ChainId,
		root: Element,
		height: BlockNumber,
	) -> Result<(), dispatch::DispatchError>;
	/// Update an edge for this anchor
	fn update_edge(
		id: TreeId,
		src_chain_id: ChainId,
		root: Element,
		height: BlockNumber,
	) -> Result<(), dispatch::DispatchError>;
}

/// Anchor trait for inspecting tree state
pub trait AnchorInspector<AccountId, CurrencyId, ChainId, TreeId, Element> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_neighbor_roots(id: TreeId) -> Result<Vec<Element>, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_neighbor_root(
		id: TreeId,
		src_chain_id: ChainId,
		target: Element,
	) -> Result<bool, dispatch::DispatchError>;

	// let is_known = Self::is_known_neighbor_root(id, src_chain_id, target)?;
	// ensure!(is_known, Error::<T, I>::InvalidNeighborWithdrawRoot);
	// Ok(())
	fn ensure_known_neighbor_root(
		id: TreeId,
		src_chain_id: ChainId,
		target: Element,
	) -> Result<(), dispatch::DispatchError>;
	/// Check if this anchor has this edge
	fn has_edge(id: TreeId, src_chain_id: ChainId) -> bool;
}
