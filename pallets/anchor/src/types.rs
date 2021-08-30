//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use frame_support::dispatch;

/// Anchor trait definition to be used in other pallets
pub trait AnchorInterface<T: Config<I>, I: 'static = ()> {
	// Creates a new anchor
	fn create(creator: T::AccountId, max_edges: u32, depth: u8) -> Result<T::TreeId, dispatch::DispatchError>;
	/// Deposit into the anchor
	fn deposit(account: T::AccountId, id: T::TreeId, leaf: T::Element) -> Result<(), dispatch::DispatchError>;
	/// Withdraw from the anchor
	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		roots: Vec<T::Element>,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
	) -> Result<(), dispatch::DispatchError>;
	/// Add an edge to this anchor
	fn add_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		height: T::BlockNumber,
	) -> Result<(), dispatch::DispatchError>;
	/// Update an edge for this anchor
	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		height: T::BlockNumber,
	) -> Result<(), dispatch::DispatchError>;
}

/// Anchor trait for inspecting tree state
pub trait AnchorInspector<T: Config<I>, I: 'static = ()> {
	/// Gets the merkle root for a tree or returns `TreeDoesntExist`
	fn get_neighbor_roots(id: T::TreeId) -> Result<Vec<T::Element>, dispatch::DispatchError>;
	/// Checks if a merkle root is in a tree's cached history or returns
	/// `TreeDoesntExist
	fn is_known_neighbor_root(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		target: T::Element,
	) -> Result<bool, dispatch::DispatchError>;
	fn ensure_known_neighbor_root(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		target: T::Element,
	) -> Result<(), dispatch::DispatchError> {
		let is_known = Self::is_known_neighbor_root(id, src_chain_id, target)?;
		ensure!(is_known, Error::<T, I>::InvalidNeighborWithdrawRoot);
		Ok(())
	}
	/// Check if this anchor has this edge
	fn has_edge(id: T::TreeId, src_chain_id: T::ChainId) -> bool;
}

#[derive(Default, Clone, Encode, Decode)]
pub struct AnchorMetadata<AccountId, Balance> {
	/// Creator account
	pub creator: AccountId,
	/// Balance size of deposit
	pub deposit_size: Balance,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, Debug)]
pub struct EdgeMetadata<ChainID, Element, BlockNumber> {
	/// chain id
	pub src_chain_id: ChainID,
	/// root of source chain anchor's native merkle tree
	pub root: Element,
	/// height of source chain anchor's native merkle tree
	pub height: BlockNumber,
}
