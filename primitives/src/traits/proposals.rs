use frame_support::dispatch;
use webb_proposals::Proposal;

pub trait OnSignedProposal {
	fn on_signed_proposal(proposal: Proposal) -> Result<(), dispatch::DispatchError>;
}
