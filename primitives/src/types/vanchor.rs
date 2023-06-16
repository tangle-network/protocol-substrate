use super::{ElementTrait, IntoAbiToken, Token};
use codec::{Decode, Encode, MaxEncodedLen};
use ethabi::{Int, Uint};
use scale_info::TypeInfo;
use sp_std::{vec, vec::Vec};

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct VAnchorMetadata<AccountId, CurrencyId> {
	/// Creator account
	pub creator: Option<AccountId>,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: CurrencyId,
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ProofData<E> {
	pub proof: Vec<u8>,
	pub public_amount: E,
	pub roots: Vec<E>,
	pub input_nullifiers: Vec<E>,
	pub output_commitments: Vec<E>,
	pub ext_data_hash: E,
}

impl<E: ElementTrait> ProofData<E> {
	pub fn new(
		proof: Vec<u8>,
		public_amount: E,
		roots: Vec<E>,
		input_nullifiers: Vec<E>,
		output_commitments: Vec<E>,
		ext_data_hash: E,
	) -> Self {
		Self { proof, public_amount, roots, input_nullifiers, output_commitments, ext_data_hash }
	}
}

#[derive(Encode, Decode, Default, Debug, Clone, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, CurrencyId: Encode> {
	pub recipient: AccountId,
	pub relayer: AccountId,
	pub ext_amount: Amount,
	pub fee: Balance,
	pub refund: Balance,
	pub token: CurrencyId,
	pub encrypted_output1: Vec<u8>,
	pub encrypted_output2: Vec<u8>,
}

impl<I: Encode, A: Encode, B: Encode, C: Encode> ExtData<I, A, B, C> {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		recipient: I,
		relayer: I,
		ext_amount: A,
		fee: B,
		refund: B,
		token: C,
		encrypted_output1: Vec<u8>,
		encrypted_output2: Vec<u8>,
	) -> Self {
		Self {
			recipient,
			relayer,
			ext_amount,
			fee,
			refund,
			token,
			encrypted_output1,
			encrypted_output2,
		}
	}
}

impl<I: Encode, A: Encode, B: Encode, C: Encode> IntoAbiToken for ExtData<I, A, B, C> {
	// (bytes recipient,int256 extAmount,bytes relayer,uint256 fee,uint256
	// refund,bytes token,bytes encryptedOutput1,bytes encryptedOutput2)
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Int(Int::from_little_endian(&self.ext_amount.encode()));
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Uint(Uint::from_little_endian(&self.fee.encode()));
		let refund = Token::Uint(Uint::from_little_endian(&self.refund.encode()));
		let token = Token::Bytes(self.token.encode());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.clone());
		let ext_data_args = vec![
			recipient,
			ext_amount,
			relayer,
			fee,
			refund,
			token,
			encrypted_output1,
			encrypted_output2,
		];
		Token::Tuple(ext_data_args)
	}
}

#[cfg(test)]
mod tests {
	use crate::hasher::InstanceHasher;
	use core::convert::TryInto;

	use super::*;

	#[test]
	fn ext_data_hash_works() {
		type AccountId = [u8; 32];
		type Amount = i128;
		type Balance = u128;
		type AssetId = u32;

		let recipient: AccountId =
			hex::decode("306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20")
				.unwrap()
				.try_into()
				.unwrap();
		let relayer: AccountId = recipient;
		let ext_amount: Amount = -1000000000000000000000;
		let fee: Balance = 0;
		let refund: Balance = 0;
		let token: AssetId = 0;
		let encrypted_output1 = hex::decode("4857e108572669341113cbe18b92defdd8ec1d2e54c7b39ea32007ce1df1232a743b9cef820f62b918825d207513b892e07908a89332c52a5fadc6bd2b5c2c8f3fad3768cb303e42bf43ecdb8779c72942401485f600c6a89cf48406fc12702e9b416bdd128b672e7d0f677aca180bb687ab2945208fdbf0d7f231d109a04d5a063c7728dd474d4709c9c6b78b20a5ad8d66ab3bf70ccce13f430fe09cca015d91d1124b3cb3a445").unwrap();
		let encrypted_output2 = hex::decode("b57f36d4a39a9a571f65d0f7c1dbe80862925f73dc6fd5ccf8e2e196c4a7d37986497f5337b3bb29137128d4310e76525371602387999724d32fbddc898c6e8234e9756e48eee766e96196ad390f48ee5b8581407f65398f2c18ecf5d1edf92c8ed33ddff666d6cf6ca36e036a09124732d060c2029ab50e3bd223b33c69f0f28e979434b2b4abc72eb3eeb62dbfb4afdde749244adec2c0b00d6ce26361e02c8e32e83dba939472").unwrap();

		let ext_data = ExtData::new(
			recipient,
			relayer,
			ext_amount,
			fee,
			refund,
			token,
			encrypted_output1,
			encrypted_output2,
		);
		let ext_data_bytes = ext_data.encode_abi();
		eprintln!("ext_data_bytes: 0x{}", hex::encode(&ext_data_bytes));
		let hash1 = crate::hashing::ethereum::keccak_256(&ext_data_bytes);
		eprintln!("hash1: 0x{}", hex::encode(hash1));
		let hash = crate::hashing::ethereum::Keccak256HasherBn254::hash(
			&ext_data_bytes,
			Default::default(),
		)
		.unwrap();
		let expected_hash = "04b18a1a64975a01e67f70e790434edd7d85bf51935580f3ddbe20e0c9abecc8";
		assert_eq!(hex::encode(hash), expected_hash);
	}
}
