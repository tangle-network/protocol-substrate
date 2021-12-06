use super::{ElementTrait, IntoAbiToken, Token};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct VAnchorMetadata<AccountId, AssetId> {
	/// Creator account
	pub creator: AccountId,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: AssetId,
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ProofData<E> {
	pub proof: Vec<u8>,
	pub roots: Vec<E>,
	pub input_nullifiers: Vec<E>,
	pub output_commitments: Vec<E>,
	pub public_amount: E,
	pub ext_data_hash: E,
}

impl<E: ElementTrait> ProofData<E> {
	pub fn new(
		proof: Vec<u8>,
		roots: Vec<E>,
		input_nullifiers: Vec<E>,
		output_commitments: Vec<E>,
		public_amount: E,
		ext_data_hash: E,
	) -> Self {
		Self {
			proof,
			roots,
			input_nullifiers,
			output_commitments,
			public_amount,
			ext_data_hash,
		}
	}
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, Element: Encode> {
	pub recipient: AccountId,
	pub relayer: AccountId,
	pub ext_amount: Amount,
	pub fee: Balance,
	pub encrypted_output1: Element,
	pub encrypted_output2: Element,
}

impl<I: Encode, A: Encode, B: Encode, E: Encode> ExtData<I, A, B, E> {
	pub fn new(recipient: I, relayer: I, ext_amount: A, fee: B, encrypted_output1: E, encrypted_output2: E) -> Self {
		Self {
			recipient,
			relayer,
			ext_amount,
			fee,
			encrypted_output1,
			encrypted_output2,
		}
	}
}

impl<I: Encode, A: Encode, B: Encode, E: Encode> IntoAbiToken for ExtData<I, A, B, E> {
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Bytes(self.ext_amount.encode());
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Bytes(self.fee.encode());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.encode());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.encode());
		let mut touple = Vec::new();
		touple.push(recipient);
		touple.push(relayer);
		touple.push(ext_amount);
		touple.push(fee);
		touple.push(encrypted_output1);
		touple.push(encrypted_output2);
		Token::Tuple(touple)
	}
}
