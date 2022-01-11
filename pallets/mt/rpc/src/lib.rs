#![allow(clippy::unnecessary_mut_passed)]

use std::sync::Arc;

use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use webb_primitives::ElementTrait;
use pallet_mt_rpc_runtime_api::MerkleTreeApi;

/// Merkle RPC methods.
#[rpc]
pub trait MerkleApi<BlockHash, Element> {
	/// Get The MerkleTree leaves.
	///
	/// This method calls into a runtime with `Merkle` pallet included and
	/// attempts to get the merkletree leaves.
	/// Optionally, a block hash at which the runtime should be queried can be
	/// specified.
	///
	/// Returns the (full) a Vec<[u8; 32]> of the leaves.
	#[rpc(name = "mt_getLeaves")]
	fn get_leaves(&self, tree_id: u32, from: usize, to: usize, at: Option<BlockHash>) -> Result<Vec<Element>>;
}

/// A struct that implements the `MerkleApi`.
pub struct MerkleClient<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> MerkleClient<C, M> {
	/// Create new `Merkle` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<C, Block, Element> MerkleApi<<Block as BlockT>::Hash, Element> for MerkleClient<C, Block>
where
	Block: BlockT,
	Element: ElementTrait,
	C: HeaderBackend<Block> + ProvideRuntimeApi<Block> + Send + Sync + 'static,
	C::Api: MerkleTreeApi<Block, Element>,
{
	fn get_leaves(
		&self,
		tree_id: u32,
		from: usize,
		to: usize,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Vec<Element>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		if to - from >= 512 {
			return Err(Error {
				code: ErrorCode::ServerError(1512), // Too many leaves
				message: "TooManyLeaves".into(),
				data: Some("MaxRange512".into()),
			});
		}
		let leaves = (from..to)
			.into_iter()
			.map(|i| api.get_leaf(&at, tree_id, i as u32)) // Result<Option<Element>>
			.flatten() // Option<Element>
			.flatten() // Element
			.collect();
		Ok(leaves)
	}
}
