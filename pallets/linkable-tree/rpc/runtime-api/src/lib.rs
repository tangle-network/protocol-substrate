#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use pallet_linkable_tree::types::EdgeMetadata;
use sp_std::vec::Vec;
use webb_primitives::ElementTrait;

// pub struct NeighborRoots<E>(Vec<E>);

// #[#[derive(scale_info::TypeInfo)]]
// pub struct NeighborEdges<C, E, L>(Vec<EdgeMetadata<C, E, L>>);

sp_api::decl_runtime_apis! {
	pub trait LinkableTreeApi<C, E, L>
	where
		C: Encode + Decode,
		E: ElementTrait,
		L: Encode + Decode,
	{
		/// Get the neighbor roots including the roots for default (empty) edges
		fn get_neighbor_roots(tree_id: u32) -> Vec<E>;
		/// Get the neighbor edge metadata including the metadata for default (empty) edges
		fn get_neighbor_edges(tree_id: u32) -> Vec<EdgeMetadata<C, E, L>>;
	}
}
