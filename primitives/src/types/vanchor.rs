use super::{IntoAbiToken, Token};
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct VAnchorMetadata<AccountId, AssetId> {
	/// Creator account
	pub creator: AccountId,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: AssetId,
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ProofData<Element, Balance> {
	pub proof: Vec<u8>,
	pub roots: Vec<Element>,
	pub input_nullifiers: Vec<Element>,
	pub output_commitments: Vec<Element>,
	pub public_amount: Balance,
	pub ext_data_hash: Element,
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, Element: Encode> {
	pub recipient: AccountId,
	pub ext_amount: Amount,
	pub relayer: AccountId,
	pub fee: Balance,
	pub encrypted_output1: Element,
	pub encrypted_output2: Element,
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
