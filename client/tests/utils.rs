use ark_std::collections::BTreeMap;
use core::fmt::Debug;
use subxt::{DefaultConfig, Event, TransactionProgress};

use subxt::sp_runtime::AccountId32;
use webb_client::webb_runtime;
use webb_runtime::runtime_types::{
	sp_runtime::DispatchError,
	webb_primitives::types::vanchor::{ExtData as WebbExtData, ProofData as WebbProofData},
	webb_standalone_runtime::Element as WebbElement,
};

use ark_ff::{BigInteger, PrimeField};

use arkworks_native_gadgets::poseidon::Poseidon;
pub use arkworks_setups::common::{
	prove, prove_unchecked, setup_tree_and_create_path, verify_unchecked_raw,
};
use arkworks_setups::{
	common::{setup_params, Leaf},
	r1cs::{anchor::AnchorR1CSProver, mixer::MixerR1CSProver, vanchor::VAnchorR1CSProver},
	utxo::Utxo,
	Curve, MixerProver, VAnchorProver,
};
use webb_primitives::types::{ElementTrait, IntoAbiToken, Token};

// wasm-utils dependencies
use ark_std::{rand::thread_rng, UniformRand};
use wasm_utils::{
	proof::{generate_proof_js, JsProofInput, MixerProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};

use ark_bn254::{Bn254, Fr as Bn254Fr};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(
	Debug, Encode, Decode, Default, Copy, Clone, PartialEq, Eq, TypeInfo, Serialize, Deserialize,
)]
pub struct Element(pub [u8; 32]);

impl ElementTrait for Element {
	fn to_bytes(&self) -> &[u8] {
		&self.0
	}

	fn from_bytes(input: &[u8]) -> Self {
		let mut buf = [0u8; 32];
		buf.iter_mut().zip(input).for_each(|(a, b)| *a = *b);
		Self(buf)
	}
}

impl Into<WebbElement> for Element {
	fn into(self) -> WebbElement {
		WebbElement(self.0)
	}
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

impl Into<WebbProofData<WebbElement>> for ProofData<Element> {
	fn into(self) -> WebbProofData<WebbElement> {
		WebbProofData {
			proof: self.proof,
			public_amount: self.public_amount.into(),
			roots: self.roots.iter().map(|x| x.clone().into()).collect(),
			input_nullifiers: self.input_nullifiers.iter().map(|x| x.clone().into()).collect(),
			output_commitments: self.output_commitments.iter().map(|x| x.clone().into()).collect(),
			ext_data_hash: self.ext_data_hash.into(),
		}
	}
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode> {
	pub recipient: AccountId,
	pub relayer: AccountId,
	pub ext_amount: Amount,
	pub fee: Balance,
	pub encrypted_output1: Vec<u8>,
	pub encrypted_output2: Vec<u8>,
}

impl<I: Encode, A: Encode, B: Encode> ExtData<I, A, B> {
	pub fn new(
		recipient: I,
		relayer: I,
		ext_amount: A,
		fee: B,
		encrypted_output1: Vec<u8>,
		encrypted_output2: Vec<u8>,
	) -> Self {
		Self { recipient, relayer, ext_amount, fee, encrypted_output1, encrypted_output2 }
	}
}

impl<I: Encode, A: Encode, B: Encode> IntoAbiToken for ExtData<I, A, B> {
	fn into_abi(&self) -> Token {
		// TODO: Make sure the encodings match the solidity side
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Bytes(self.ext_amount.encode());
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Bytes(self.fee.encode());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.clone());
		let mut ext_data_args = Vec::new();
		ext_data_args.push(recipient);
		ext_data_args.push(relayer);
		ext_data_args.push(ext_amount);
		ext_data_args.push(fee);
		ext_data_args.push(encrypted_output1);
		ext_data_args.push(encrypted_output2);
		Token::Tuple(ext_data_args)
	}
}

impl Into<WebbExtData<AccountId32, i128, u128>> for ExtData<AccountId32, i128, u128> {
	fn into(self) -> WebbExtData<AccountId32, i128, u128> {
		WebbExtData {
			recipient: self.recipient.clone(),
			relayer: self.relayer.clone(),
			ext_amount: self.ext_amount,
			fee: self.fee,
			encrypted_output1: self.encrypted_output1,
			encrypted_output2: self.encrypted_output2,
		}
	}
}

const TREE_DEPTH: usize = 30;
const ANCHOR_CT: usize = 2;
pub const NUM_UTXOS: usize = 2;
pub const DEFAULT_LEAF: [u8; 32] = [
	108, 175, 153, 72, 237, 133, 150, 36, 226, 65, 231, 118, 15, 52, 27, 130, 180, 93, 161, 235,
	182, 53, 58, 52, 243, 171, 172, 211, 96, 76, 229, 47,
];

#[allow(non_camel_case_types)]
type MixerProver_Bn254_30 = MixerR1CSProver<Bn254, TREE_DEPTH>;
#[allow(non_camel_case_types)]
type VAnchorProver_Bn254_30_2_2_2 =
	VAnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT, NUM_UTXOS, NUM_UTXOS>;

pub fn setup_mixer_leaf() -> (Element, Element, Element, Element) {
	let rng = &mut thread_rng();
	let curve = Curve::Bn254;
	let secret = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
		MixerProver_Bn254_30::create_leaf_with_privates(curve, secret, nullifier).unwrap();

	let leaf_element = Element::from_bytes(&leaf_bytes);
	let secret_element = Element::from_bytes(&secret_bytes);
	let nullifier_element = Element::from_bytes(&nullifier_bytes);
	let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);

	(leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn setup_mixer_circuit(
	leaves: Vec<Vec<u8>>,
	leaf_index: u64,
	secret: Vec<u8>,
	nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
	pk_bytes: Vec<u8>,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
	let mixer_proof_input = MixerProofInput {
		exponentiation: 5,
		width: 3,
		curve: WasmCurve::Bn254,
		backend: Backend::Arkworks,
		secret,
		nullifier,
		recipient: recipient_bytes,
		relayer: relayer_bytes,
		pk: pk_bytes,
		refund: refund_value,
		fee: fee_value,
		chain_id: 0,
		leaves,
		leaf_index,
	};
	let js_proof_inputs = JsProofInput { inner: ProofInput::Mixer(mixer_proof_input) };
	let proof = generate_proof_js(js_proof_inputs).unwrap();

	let root_array: [u8; 32] = proof.root.try_into().unwrap();
	let root_element = Element(root_array);

	(proof.proof, root_element)
}

pub fn setup_utxos(
	// Transaction inputs
	chain_ids: [u64; NUM_UTXOS],
	amounts: [u128; NUM_UTXOS],
	indices: Option<[u64; NUM_UTXOS]>,
) -> [Utxo<Bn254Fr>; NUM_UTXOS] {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();
	// Input Utxos
	let indices: [Option<u64>; NUM_UTXOS] = if indices.is_some() {
		let ind_unw = indices.unwrap();
		ind_unw.map(|x| Some(x))
	} else {
		[None; NUM_UTXOS]
	};
	let utxo1 = VAnchorProver_Bn254_30_2_2_2::create_random_utxo(
		curve,
		chain_ids[0],
		amounts[0],
		indices[0],
		rng,
	)
	.unwrap();
	let utxo2 = VAnchorProver_Bn254_30_2_2_2::create_random_utxo(
		curve,
		chain_ids[1],
		amounts[1],
		indices[1],
		rng,
	)
	.unwrap();
	let in_utxos = [utxo1, utxo2];

	in_utxos
}

pub fn setup_vanchor_circuit(
	// Metadata inputs
	public_amount: i128,
	chain_id: u64,
	ext_data_hash: Vec<u8>,
	in_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	out_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	custom_roots: Option<[Vec<u8>; ANCHOR_CT]>,
	leaves: Vec<Vec<u8>>,
	pk_bytes: Vec<u8>,
) -> (Vec<u8>, Vec<Bn254Fr>) {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();

	let leaves_f: Vec<Bn254Fr> =
		leaves.iter().map(|x| Bn254Fr::from_le_bytes_mod_order(&x)).collect();

	let mut in_leaves: BTreeMap<u64, Vec<Vec<u8>>> = BTreeMap::new();
	in_leaves.insert(chain_id, leaves);
	let in_indices = [in_utxos[0].get_index().unwrap(), in_utxos[1].get_index().unwrap()];

	// This allows us to pass zero roots for initial transaction
	let in_root_set = if custom_roots.is_some() {
		custom_roots.unwrap()
	} else {
		let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
		let poseidon3 = Poseidon::new(params3);
		let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
			&poseidon3,
			&leaves_f,
			0,
			&DEFAULT_LEAF,
		)
		.unwrap();
		[(); ANCHOR_CT].map(|_| tree.root().into_repr().to_bytes_le())
	};

	let vanchor_proof = VAnchorProver_Bn254_30_2_2_2::create_proof(
		curve,
		chain_id,
		public_amount,
		ext_data_hash,
		in_root_set,
		in_indices,
		in_leaves,
		in_utxos,
		out_utxos,
		pk_bytes.clone(),
		DEFAULT_LEAF,
		rng,
	)
	.unwrap();

	let pub_ins = vanchor_proof
		.public_inputs_raw
		.iter()
		.map(|x| Bn254Fr::from_le_bytes_mod_order(x))
		.collect();

	(vanchor_proof.proof, pub_ins)
}

pub fn deconstruct_vanchor_pi(
	public_inputs: &Vec<Bn254Fr>,
) -> (
	Bn254Fr,      // Chain Id
	Bn254Fr,      // Public amount
	Vec<Bn254Fr>, // Roots
	Vec<Bn254Fr>, // Input tx Nullifiers
	Vec<Bn254Fr>, // Output tx commitments
	Bn254Fr,      // External data hash
) {
	let public_amount = public_inputs[0];
	let ext_data_hash = public_inputs[1];
	let nullifiers = public_inputs[2..4].to_vec();
	let commitments = public_inputs[4..6].to_vec();
	let chain_id = public_inputs[6];
	let root_set = public_inputs[7..9].to_vec();
	(chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash)
}

pub fn deconstruct_vanchor_pi_el(
	public_inputs_f: &Vec<Bn254Fr>,
) -> (
	Element,      // Chain Id
	Element,      // Public amount
	Vec<Element>, // Roots
	Vec<Element>, // Input tx Nullifiers
	Vec<Element>, // Output tx commitments
	Element,      // External amount
) {
	let (chain_id, public_amount, roots, nullifiers, commitments, ext_data_hash) =
		deconstruct_vanchor_pi(public_inputs_f);
	let chain_id_el = Element::from_bytes(&chain_id.into_repr().to_bytes_le());
	let public_amount_el = Element::from_bytes(&public_amount.into_repr().to_bytes_le());
	let root_set_el = roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let nullifiers_el = nullifiers
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let commitments_el = commitments
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_le());
	(chain_id_el, public_amount_el, root_set_el, nullifiers_el, commitments_el, ext_data_hash_el)
}

pub async fn expect_event<E: Event + Debug>(
	tx_progess: &mut TransactionProgress<
		'_,
		DefaultConfig,
		DispatchError,
		webb_client::webb_runtime::Event,
	>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Start printing on a fresh line
	println!("");

	while let Some(ev) = tx_progess.next_item().await {
		let ev = ev?;
		use subxt::TransactionStatus::*;

		// Made it into a block, but not finalized.
		if let InBlock(details) = ev {
			println!(
				"Transaction {:?} made it into block {:?}",
				details.extrinsic_hash(),
				details.block_hash()
			);

			let events = details.wait_for_success().await?;
			let transfer_event = events.find_first::<E>()?;

			if let Some(event) = transfer_event {
				println!("In block (but not finalized): {event:?}");
			} else {
				println!("Failed to find Event");
			}
		}
		// Finalized!
		else if let Finalized(details) = ev {
			println!(
				"Transaction {:?} is finalized in block {:?}",
				details.extrinsic_hash(),
				details.block_hash()
			);

			let events = details.wait_for_success().await?;
			let transfer_event = events.find_first::<E>()?;

			if let Some(event) = transfer_event {
				println!("Transaction success: {event:?}");
			} else {
				println!("Failed to find Balances::Transfer Event");
			}
		}
		// Report other statuses we see.
		else {
			println!("Current transaction status: {:?}", ev);
		}
	}

	Ok(().into())
}

pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
