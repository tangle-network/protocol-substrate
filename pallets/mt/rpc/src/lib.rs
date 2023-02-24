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

use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sc_rpc::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

use pallet_mt_rpc_runtime_api::MerkleTreeApi;
use webb_primitives::ElementTrait;

/// Merkle RPC methods.
#[rpc(client, server)]
pub trait MerkleTreeRpcApi<BlockHash, Element> {
	/// Get The MerkleTree leaves.
	///
	/// This method calls into a runtime with `Merkle` pallet included and
	/// attempts to get the merkletree leaves.
	/// Optionally, a block hash at which the runtime should be queried can be
	/// specified.
	///
	/// Returns the (full) a Vec<[u8; 32]> of the leaves.
	#[method(name = "mt_getLeaves")]
	fn get_leaves(
		&self,
		tree_id: u32,
		from: usize,
		to: usize,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<Element>>;
}

/// A struct that implements the `MerkleTreeRpcApi`.
pub struct MerkleTreeClient<C, M> {
	client: Arc<C>,
	deny_unsafe: DenyUnsafe,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> MerkleTreeClient<C, M> {
	/// Create new `Merkle` instance with the given reference to the client.
	pub fn new(client: Arc<C>, deny_unsafe: DenyUnsafe) -> Self {
		Self { client, deny_unsafe, _marker: Default::default() }
	}
}

impl<C, Block, Element> MerkleTreeRpcApiServer<<Block as BlockT>::Hash, Element>
	for MerkleTreeClient<C, Block>
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
	) -> RpcResult<Vec<Element>> {
		self.deny_unsafe.check_if_safe()?;

		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		if to - from >= 512 {
			return Err(error::Error::TooManyLeavesRequested.into());
		}
		let leaves = (from..to)
			.into_iter()
			.flat_map(|i| api.get_leaf(&at, tree_id, i as u32)) // Result<Option<Element>>
			.flatten() // Element
			.collect();
		Ok(leaves)
	}
}
