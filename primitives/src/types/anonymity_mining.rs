use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

use crate::ElementTrait;

#[derive(Clone, Encode, Decode, Debug, Default, Eq, PartialEq, TypeInfo)]
pub struct RewardProofData<E: ElementTrait> {
	pub proof: Vec<u8>,
	pub rate: E,
	pub fee: E,
	pub reward_nullifier: E,
	pub note_ak_alpha_x: E,
	pub note_ak_alpha_y: E,
	pub ext_data_hash: E,
	pub input_root: E,
	pub input_nullifier: E,
	pub output_commitment: E,
	pub spent_roots: Vec<E>,
	pub unspent_roots: Vec<E>,
}
