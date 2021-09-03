use codec::{Decode, Encode};

pub mod keystore;

#[derive(Debug, Decode, Encode)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub enum DKGType {
	MultiPartyECDSA,
}

/// WEBB DKG (distributed key generation) message.
///
/// A vote message is a direct vote created by a WEBB node on every voting round
/// and is gossiped to its peers.
#[derive(Debug, Decode, Encode)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct DKGMessage<Id> {
	/// Node authority id
	pub id: Id,
	/// DKG protocol type identifier
	pub dkg_type: DKGType,
	/// DKG message contents
	pub message: Vec<u8>,
}

impl<Id> DKGMessage<Id> {
	fn new(id: Id, dkg_type: DKGType, message: Vec<u8>) -> Self {
		Self { id, dkg_type, message }
	}
}