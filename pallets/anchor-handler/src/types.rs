use codec::{Decode, Encode};
use pallet_anchor::types::EdgeMetadata;
use scale_info::TypeInfo;

#[derive(Default, Clone, Encode, Decode, PartialEq, Debug, TypeInfo)]
pub struct UpdateRecord<TreeId, ResourceId, ChainID, Element, BlockNumber> {
	pub tree_id: TreeId,
	pub resource_id: ResourceId,
	pub edge_metadata: EdgeMetadata<ChainID, Element, BlockNumber>,
}
