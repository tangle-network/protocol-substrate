use scale_info::TypeInfo;

use crate::*;
pub trait DemocracyGovernanceDelegate<T: SystemConfig, Proposal, Balance> {
	fn propose(origin: OriginFor<T>, proposal: Proposal, value: Balance) -> DispatchResult;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub struct LinkProposal<ChainId, TreeId> {
	pub target_chain_id: ChainId,
	pub target_tree_id: TreeId,
	pub local_tree_id: TreeId,
}
