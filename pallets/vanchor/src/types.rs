//! All the traits exposed to be used in other custom pallets
use crate::*;
use codec::{Decode, Encode};
use darkwebb_primitives::types::{IntoAbiToken, Token};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct VAnchorMetadata<AccountId, AssetId> {
	/// Creator account
	pub creator: AccountId,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: AssetId,
}

struct Proof<Element, Balance> {
	proof: Vec<u8>,
	roots: Vec<Element>,
	input_nullifiers: Vec<Element>,
	output_commitments: Vec<Element>,
	public_amount: Balance,
	ext_data_hash: Element,
}

pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, Element: Encode> {
	recipient: AccountId,
	ext_amount: Amount,
	relayer: AccountId,
	fee: Balance,
	encrypted_output1: Element,
	encrypted_output2: Element,
}

impl<I: Encode, A: Encode, B: Encode, E: Encode> IntoAbiToken for ExtData<I, A, B, E> {
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Int(self.ext_amount.encode().as_slice().into());
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Uint(self.fee.encode().as_slice().into());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.encode());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.encode());
		Token::Tuple(vec![
			recipient,
			ext_amount,
			relayer,
			fee,
			encrypted_output1,
			encrypted_output2,
		])
	}
}
