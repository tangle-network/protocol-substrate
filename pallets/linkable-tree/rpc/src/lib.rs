// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(clippy::unnecessary_mut_passed)]

mod error;

use std::sync::Arc;

use codec::{Decode, Encode};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sc_rpc::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use pallet_linkable_tree::types::EdgeMetadata;
use pallet_linkable_tree_rpc_runtime_api::LinkableTreeApi;
use webb_primitives::ElementTrait;

/// Linkable Tree RPC methods.
#[rpc(client, server)]
pub trait LinkableTreeRpcApi<BlockHash, E, C, L> {
	/// Get the Linkable Tree neighbor roots.
	///
	/// Returns the (full) Vec<Element> of the neighbor roots
	#[method(name = "lt_getNeighborRoots")]
	fn get_neighbor_roots(&self, tree_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<E>>;

	/// Get the Linkable Tree neighbor edges.
	///
	/// Returns the (full) Vec<EdgeMetadata> of the neighbor edge metadata
	#[method(name = "lt_getNeighborEdges")]
	fn get_neighbor_edges(
		&self,
		tree_id: u32,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<EdgeMetadata<C, E, L>>>;
}

/// A struct that implements the `LinkableTreeApi`.
pub struct LinkableTreeClient<C, M> {
	client: Arc<C>,
	deny_unsafe: DenyUnsafe,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> LinkableTreeClient<C, M> {
	/// Create new `Merkle` instance with the given reference to the client.
	pub fn new(client: Arc<C>, deny_unsafe: DenyUnsafe) -> Self {
		Self { client, deny_unsafe, _marker: Default::default() }
	}
}

impl<C, B, E, CID, L> LinkableTreeRpcApiServer<<B as BlockT>::Hash, E, CID, L>
	for LinkableTreeClient<C, B>
where
	B: BlockT,
	E: ElementTrait,
	CID: Encode + Decode,
	L: Encode + Decode,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: LinkableTreeApi<B, CID, E, L>,
{
	fn get_neighbor_roots(
		&self,
		tree_id: u32,
		at: Option<<B as BlockT>::Hash>,
	) -> RpcResult<Vec<E>> {
		self.deny_unsafe.check_if_safe()?;

		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_neighbor_roots(at, tree_id)
			.map_err(|_| error::Error::RootsRequestFailure)
			.map_err(Into::into)
	}

	fn get_neighbor_edges(
		&self,
		tree_id: u32,
		at: Option<<B as BlockT>::Hash>,
	) -> RpcResult<Vec<EdgeMetadata<CID, E, L>>> {
		self.deny_unsafe.check_if_safe()?;

		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_neighbor_edges(at, tree_id)
			.map_err(|_| error::Error::EdgesRequestFailure)
			.map_err(Into::into)
	}
}
