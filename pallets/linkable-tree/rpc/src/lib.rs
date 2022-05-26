#![allow(clippy::unnecessary_mut_passed)]

use std::sync::Arc;

use codec::{Decode, Encode};
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use pallet_linkable_tree::types::EdgeMetadata;
use pallet_linkable_tree_rpc_runtime_api::LinkableTreeApi;
use webb_primitives::ElementTrait;

/// Linkable Tree RPC methods.
#[rpc]
pub trait LinkableTreeRpcApi<BlockHash, E, C, L> {
	/// Get the Linkable Tree neighbor roots.
	///
	/// Returns the (full) Vec<Element> of the neighbor roots
	#[rpc(name = "lt_getNeighborRoots")]
	fn get_neighbor_roots(&self, tree_id: u32, at: Option<BlockHash>) -> Result<Vec<E>>;

	/// Get the Linkable Tree neighbor edges.
	///
	/// Returns the (full) Vec<EdgeMetadata> of the neighbor edge metadata
	#[rpc(name = "lt_getNeighborEdges")]
	fn get_neighbor_edges(
		&self,
		tree_id: u32,
		at: Option<BlockHash>,
	) -> Result<Vec<EdgeMetadata<C, E, L>>>;
}

/// A struct that implements the `LinkableTreeApi`.
pub struct LinkableTreeClient<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> LinkableTreeClient<C, M> {
	/// Create new `Merkle` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, B, E, CID, L> LinkableTreeRpcApi<<B as BlockT>::Hash, E, CID, L>
	for LinkableTreeClient<C, B>
where
	B: BlockT,
	E: ElementTrait,
	CID: Encode + Decode,
	L: Encode + Decode,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: LinkableTreeApi<B, CID, E, L>,
{
	fn get_neighbor_roots(&self, tree_id: u32, at: Option<<B as BlockT>::Hash>) -> Result<Vec<E>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		api.get_neighbor_roots(&at, tree_id).map_err(|e| {
			return Error {
				code: ErrorCode::ServerError(1513), // Too many leaves
				message: "NoNeighborRoots".into(),
				data: Some(format!("{:?}", e).into()),
			}
		})
	}

	fn get_neighbor_edges(
		&self,
		tree_id: u32,
		at: Option<<B as BlockT>::Hash>,
	) -> Result<Vec<EdgeMetadata<CID, E, L>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		api.get_neighbor_edges(&at, tree_id).map_err(|e| {
			return Error {
				code: ErrorCode::ServerError(1513), // Too many leaves
				message: "NoNeighborEdges".into(),
				data: Some(format!("{:?}", e).into()),
			}
		})
	}
}
