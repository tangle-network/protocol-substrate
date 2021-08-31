use codec::{Decode, Encode};
use pallet_anchor::types::EdgeMetadata;
use pallet_bridge::types::{ChainId, ResourceId};
use pallet_mt::types::ElementTrait;

use sp_std::prelude::*;

#[derive(Default, Clone, Encode, Decode)]
pub struct UpdateRecord<TreeId, ResourceId, ChainID, Element, BlockNumber> {
	pub tree_id: TreeId,
	pub resource_id: ResourceId,
	pub edge_metadata: EdgeMetadata<ChainID, Element, BlockNumber>,
}
