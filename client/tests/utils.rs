use ark_std::{collections::BTreeMap, rand::rngs::OsRng};
use core::fmt::Debug;
use subxt::{
	client::OnlineClientT,
	events::StaticEvent,
	ext::sp_runtime::AccountId32,
	tx::{TxProgress, TxStatus, TxStatus::*},
	Config,
};
use webb_client::webb_runtime::{
	self, runtime_types::webb_primitives::runtime::Element as WebbElement,
};
use webb_runtime::runtime_types::webb_primitives::types::vanchor::{
	ExtData as WebbExtData, ProofData as WebbProofData,
};

use ark_ff::{BigInteger, PrimeField};

use arkworks_native_gadgets::poseidon::Poseidon;
pub use arkworks_setups::common::{
	prove, prove_unchecked, setup_tree_and_create_path, verify_unchecked_raw,
};
use arkworks_setups::{
	common::{setup_params, Leaf},
	r1cs::{mixer::MixerR1CSProver, vanchor::VAnchorR1CSProver},
	utxo::Utxo,
	Curve, MixerProver, VAnchorProver,
};
use webb_primitives::types::{ElementTrait, IntoAbiToken, Token};

use ark_bn254::{Bn254, Fr};
use ark_circom::WitnessCalculator;
use ark_groth16::{
	create_proof_with_reduction_and_matrices, prepare_verifying_key,
	verify_proof as ark_verify_proof, Proof as ArkProof, ProvingKey, VerifyingKey,
};
use ark_relations::r1cs::ConstraintMatrices;
use ark_std::{rand::thread_rng, vec::Vec, UniformRand};
use codec::{Decode, Encode};
use num_bigint::BigInt;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, sync::Mutex};

type Bn254Fr = ark_bn254::Fr;

pub const DEFAULT_LEAF: [u8; 32] = [
	47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52, 15, 118,
	231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
];

#[allow(non_camel_case_types)]
type VAnchorProver_Bn254_30_2_2_2 =
	VAnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT, NUM_UTXOS, NUM_UTXOS>;

use ark_bn254::{Fq, Fq2, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_circom::{read_zkey, CircomConfig, CircomReduction};
use ark_ff::{BigInteger256, ToBytes};
use ark_relations::r1cs::SynthesisError;
use arkworks_native_gadgets::merkle_tree::{Path, SparseMerkleTree};
use cfg_if::cfg_if;
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::OnInitialize};
use num_bigint::{BigUint, Sign};
use once_cell::sync::OnceCell;
use serde_json::Value;
use sp_core::hashing::keccak_256;
use std::{
	convert::TryFrom,
	fs::{self, File},
	io::{Cursor, Error, ErrorKind},
	result::Result,
	str::FromStr,
};
use thiserror::Error;
use wasmer::{Module, Store};
use webb_primitives::{
	linkable_tree::LinkableTreeInspector, merkle_tree::TreeInspector, utils::compute_chain_id_type,
	verifying::CircomError, AccountId,
};

#[derive(Error, Debug)]
pub enum ProofError {
	#[error("Error reading circuit key: {0}")]
	CircuitKeyError(#[from] std::io::Error),
	#[error("Error producing witness: {0}")]
	WitnessError(color_eyre::Report),
	#[error("Error producing proof: {0}")]
	SynthesisError(#[from] SynthesisError),
}

#[cfg(not(target_arch = "wasm32"))]
static WITNESS_CALCULATOR: OnceCell<Mutex<WitnessCalculator>> = OnceCell::new();

// Utilities to convert a json verification key in a groth16::VerificationKey
fn fq_from_str(s: &str) -> Fq {
	Fq::try_from(BigUint::from_str(s).unwrap()).unwrap()
}

// Extracts the element in G1 corresponding to its JSON serialization
fn json_to_g1(json: &Value, key: &str) -> G1Affine {
	let els: Vec<String> = json
		.get(key)
		.unwrap()
		.as_array()
		.unwrap()
		.iter()
		.map(|i| i.as_str().unwrap().to_string())
		.collect();
	G1Affine::from(G1Projective::new(
		fq_from_str(&els[0]),
		fq_from_str(&els[1]),
		fq_from_str(&els[2]),
	))
}

// Extracts the vector of G1 elements corresponding to its JSON serialization
fn json_to_g1_vec(json: &Value, key: &str) -> Vec<G1Affine> {
	let els: Vec<Vec<String>> = json
		.get(key)
		.unwrap()
		.as_array()
		.unwrap()
		.iter()
		.map(|i| {
			i.as_array()
				.unwrap()
				.iter()
				.map(|x| x.as_str().unwrap().to_string())
				.collect::<Vec<String>>()
		})
		.collect();

	els.iter()
		.map(|coords| {
			G1Affine::from(G1Projective::new(
				fq_from_str(&coords[0]),
				fq_from_str(&coords[1]),
				fq_from_str(&coords[2]),
			))
		})
		.collect()
}

// Extracts the element in G2 corresponding to its JSON serialization
fn json_to_g2(json: &Value, key: &str) -> G2Affine {
	let els: Vec<Vec<String>> = json
		.get(key)
		.unwrap()
		.as_array()
		.unwrap()
		.iter()
		.map(|i| {
			i.as_array()
				.unwrap()
				.iter()
				.map(|x| x.as_str().unwrap().to_string())
				.collect::<Vec<String>>()
		})
		.collect();

	let x = Fq2::new(fq_from_str(&els[0][0]), fq_from_str(&els[0][1]));
	let y = Fq2::new(fq_from_str(&els[1][0]), fq_from_str(&els[1][1]));
	let z = Fq2::new(fq_from_str(&els[2][0]), fq_from_str(&els[2][1]));
	G2Affine::from(G2Projective::new(x, y, z))
}

// Converts JSON to a VerifyingKey
fn to_verifying_key(json: serde_json::Value) -> VerifyingKey<Bn254> {
	VerifyingKey {
		alpha_g1: json_to_g1(&json, "vk_alpha_1"),
		beta_g2: json_to_g2(&json, "vk_beta_2"),
		gamma_g2: json_to_g2(&json, "vk_gamma_2"),
		delta_g2: json_to_g2(&json, "vk_delta_2"),
		gamma_abc_g1: json_to_g1_vec(&json, "IC"),
	}
}

// Computes the verification key from its JSON serialization
fn vk_from_json(vk_path: &str) -> VerifyingKey<Bn254> {
	let json = std::fs::read_to_string(vk_path).unwrap();
	let json: Value = serde_json::from_str(&json).unwrap();

	to_verifying_key(json)
}

pub fn generate_proof(
	#[cfg(not(target_arch = "wasm32"))] witness_calculator: &Mutex<WitnessCalculator>,
	#[cfg(target_arch = "wasm32")] witness_calculator: &mut WitnessCalculator,
	proving_key: &(ProvingKey<Bn254>, ConstraintMatrices<Fr>),
	vanchor_witness: [(&str, Vec<BigInt>); 15],
) -> Result<(ArkProof<Bn254>, Vec<Fr>), ProofError> {
	let inputs = vanchor_witness
		.into_iter()
		.map(|(name, values)| (name.to_string(), values.clone()));

	println!("inputs {:?}", inputs);

	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			let full_assignment = witness_calculator
			.calculate_witness_element::<Bn254, _>(inputs, false)
			.map_err(ProofError::WitnessError)?;
		} else {
			let full_assignment = witness_calculator
			.lock()
			.expect("witness_calculator mutex should not get poisoned")
			.calculate_witness_element::<Bn254, _>(inputs, false)
			.map_err(ProofError::WitnessError)?;
		}
	}

	// Random Values
	let mut rng = thread_rng();
	let r = Fr::rand(&mut rng);
	let s = Fr::rand(&mut rng);

	let proof = create_proof_with_reduction_and_matrices::<_, CircomReduction>(
		&proving_key.0,
		r,
		s,
		&proving_key.1,
		proving_key.1.num_instance_variables,
		proving_key.1.num_constraints,
		full_assignment.as_slice(),
	)?;

	Ok((proof, full_assignment))
}

/// Verifies a given RLN proof
///
/// # Errors
///
/// Returns a [`ProofError`] if verifying fails. Verification failure does not
/// necessarily mean the proof is incorrect.
pub fn verify_proof(
	verifying_key: &VerifyingKey<Bn254>,
	proof: &ArkProof<Bn254>,
	inputs: &Vec<Fr>,
) -> Result<bool, ProofError> {
	// Check that the proof is valid
	let pvk = prepare_verifying_key(verifying_key);
	//let pr: ArkProof<Curve> = (*proof).into();

	let verified = ark_verify_proof(&pvk, proof, inputs)?;

	Ok(verified)
}

// Initializes the witness calculator using a bytes vector
#[cfg(not(target_arch = "wasm32"))]
pub fn circom_from_raw(wasm_buffer: Vec<u8>) -> &'static Mutex<WitnessCalculator> {
	WITNESS_CALCULATOR.get_or_init(|| {
		let store = Store::default();
		let module = Module::new(&store, wasm_buffer).unwrap();
		let result =
			WitnessCalculator::from_module(module).expect("Failed to create witness calculator");
		Mutex::new(result)
	})
}

// Initializes the witness calculator
#[cfg(not(target_arch = "wasm32"))]
pub fn circom_from_folder(wasm_path: &str) -> &'static Mutex<WitnessCalculator> {
	// We read the wasm file
	let wasm_buffer = std::fs::read(wasm_path).unwrap();
	circom_from_raw(wasm_buffer)
}

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

impl From<Element> for WebbElement {
	fn from(val: Element) -> Self {
		WebbElement(val.0)
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

impl From<ProofData<Element>> for WebbProofData<WebbElement> {
	fn from(val: ProofData<Element>) -> Self {
		WebbProofData {
			proof: val.proof,
			public_amount: val.public_amount.into(),
			roots: val.roots.iter().map(|x| (*x).into()).collect(),
			input_nullifiers: val.input_nullifiers.iter().map(|x| (*x).into()).collect(),
			output_commitments: val.output_commitments.iter().map(|x| (*x).into()).collect(),
			ext_data_hash: val.ext_data_hash.into(),
		}
	}
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, AssetId> {
	pub recipient: AccountId,
	pub relayer: AccountId,
	pub ext_amount: Amount,
	pub fee: Balance,
	pub refund: Balance,
	pub token: AssetId,
	pub encrypted_output1: Vec<u8>,
	pub encrypted_output2: Vec<u8>,
}

impl<I: Encode, A: Encode, B: Encode, C: Encode> ExtData<I, A, B, C> {
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
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Bytes(self.ext_amount.encode());
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Bytes(self.fee.encode());
		let refund = Token::Bytes(self.refund.encode());
		let token = Token::Bytes(self.token.encode());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.clone());
		let mut ext_data_args = Vec::new();
		ext_data_args.push(recipient);
		ext_data_args.push(relayer);
		ext_data_args.push(ext_amount);
		ext_data_args.push(fee);
		ext_data_args.push(refund);
		ext_data_args.push(token);
		ext_data_args.push(encrypted_output1);
		ext_data_args.push(encrypted_output2);
		Token::Tuple(ext_data_args)
	}
}

impl From<ExtData<AccountId32, i128, u128, u32>> for WebbExtData<AccountId32, i128, u128, u32> {
	fn from(val: ExtData<AccountId32, i128, u128, u32>) -> Self {
		WebbExtData {
			recipient: val.recipient.clone(),
			relayer: val.relayer.clone(),
			ext_amount: val.ext_amount,
			fee: val.fee,
			refund: val.refund,
			token: val.token,
			encrypted_output1: val.encrypted_output1,
			encrypted_output2: val.encrypted_output2,
		}
	}
}

const TREE_DEPTH: usize = 30;
const ANCHOR_CT: usize = 2;
pub const NUM_UTXOS: usize = 2;

#[allow(non_camel_case_types)]
type MixerProver_Bn254_30 = MixerR1CSProver<Bn254, TREE_DEPTH>;

pub fn setup_mixer_leaf() -> (Element, Element, Element, Element) {
	let rng = &mut thread_rng();
	let curve = Curve::Bn254;
	let secret = Bn254Fr::rand(rng).into_repr().to_bytes_be();
	let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_be();
	let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
		MixerProver_Bn254_30::create_leaf_with_privates(curve, secret, nullifier).unwrap();

	let leaf_element = Element::from_bytes(&leaf_bytes);
	let secret_element = Element::from_bytes(&secret_bytes);
	let nullifier_element = Element::from_bytes(&nullifier_bytes);
	let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);

	(leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn create_mixer_proof(
	leaves: Vec<Vec<u8>>,
	leaf_index: u64,
	secret: Vec<u8>,
	nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
	pk_bytes: Vec<u8>,
	rng: &mut OsRng,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
	let mixer_proof = MixerProver_Bn254_30::create_proof(
		Curve::Bn254,
		secret,
		nullifier,
		leaves,
		leaf_index,
		recipient_bytes,
		relayer_bytes,
		fee_value,
		refund_value,
		pk_bytes,
		DEFAULT_LEAF,
		rng,
	)
	.unwrap();

	(mixer_proof.proof, Element::from_bytes(&mixer_proof.root_raw))
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
		ind_unw.map(Some)
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

	[utxo1, utxo2]
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
	circom_params: &(ProvingKey<Bn254>, ConstraintMatrices<Bn254Fr>),
	#[cfg(not(target_arch = "wasm32"))] wc: &Mutex<WitnessCalculator>,
	#[cfg(target_arch = "wasm32")] wc: &mut WitnessCalculator,
) -> (ArkProof<Bn254>, Vec<Bn254Fr>) {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();

	let leaves_f: Vec<Bn254Fr> =
		leaves.iter().map(|x| Bn254Fr::from_be_bytes_mod_order(x)).collect();

	let mut in_leaves: BTreeMap<u64, Vec<Vec<u8>>> = BTreeMap::new();
	in_leaves.insert(chain_id, leaves);
	let in_indices = [in_utxos[0].get_index().unwrap(), in_utxos[1].get_index().unwrap()];

	// This allows us to pass zero roots for initial transaction
	let (in_root_set, in_paths) = {
		let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
		let poseidon3 = Poseidon::new(params3);
		let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
			&poseidon3,
			&leaves_f,
			0,
			&DEFAULT_LEAF,
		)
		.unwrap();
		let in_paths: Vec<_> =
			in_indices.iter().map(|i| tree.generate_membership_proof(*i)).collect();
		([(); ANCHOR_CT].map(|_| tree.root().into_repr().to_bytes_be()), in_paths)
	};

	let params4 = setup_params::<Bn254Fr>(Curve::Bn254, 5, 4);
	let nullifier_hasher = Poseidon::<Bn254Fr> { params: params4 };

	// Make Inputs
	let mut public_amount_as_vec = if public_amount > 0 {
		vec![BigInt::from_bytes_be(Sign::Plus, &public_amount.to_be_bytes())]
	} else {
		vec![BigInt::from_bytes_be(Sign::Minus, &(-public_amount).to_be_bytes())]
	};

	let mut ext_data_hash_as_vec = vec![BigInt::from_bytes_be(Sign::Plus, &ext_data_hash)];
	let mut input_nullifier_as_vec = Vec::new();
	let mut output_commitment_as_vec = Vec::new();
	for i in 0..NUM_UTXOS {
		input_nullifier_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&in_utxos[i]
				.calculate_nullifier(&nullifier_hasher)
				.unwrap()
				.into_repr()
				.to_bytes_be(),
		));
		output_commitment_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&out_utxos[i].commitment.into_repr().to_bytes_be(),
		));
	}

	let mut chain_id_as_vec = vec![BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes())];

	let mut roots_as_vec = in_root_set
		.iter()
		.map(|x| BigInt::from_bytes_be(Sign::Plus, x))
		.collect::<Vec<BigInt>>();

	let mut in_amount_as_vec = Vec::new();
	let mut in_private_key_as_vec = Vec::new();
	let mut in_blinding_as_vec = Vec::new();
	let mut in_path_indices_as_vec = Vec::new();
	let mut in_path_elements_as_vec = Vec::new();
	let mut out_chain_id_as_vec = Vec::new();
	let mut out_amount_as_vec = Vec::new();
	let mut out_pub_key_as_vec = Vec::new();
	let mut out_blinding_as_vec = Vec::new();

	for i in 0..NUM_UTXOS {
		in_amount_as_vec
			.push(BigInt::from_bytes_be(Sign::Plus, &in_utxos[i].amount.into_repr().to_bytes_be()));
		in_private_key_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&in_utxos[i].keypair.secret_key.unwrap().into_repr().to_bytes_be(),
		));
		in_blinding_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&in_utxos[i].blinding.into_repr().to_bytes_be(),
		));
		in_path_indices_as_vec.push(BigInt::from(in_utxos[i].get_index().unwrap()));
		for j in 0..TREE_DEPTH {
			let neighbor_elt: Bn254Fr =
				if in_indices[i] == 0 { in_paths[i].path[j].1 } else { in_paths[i].path[j].0 };
			in_path_elements_as_vec
				.push(BigInt::from_bytes_be(Sign::Plus, &neighbor_elt.into_repr().to_bytes_be()));
		}

		out_chain_id_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&out_utxos[i].chain_id.into_repr().to_bytes_be(),
		));

		out_amount_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&out_utxos[i].amount.into_repr().to_bytes_be(),
		));

		out_pub_key_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&out_utxos[i].keypair.public_key.into_repr().to_bytes_be(),
		));

		out_blinding_as_vec.push(BigInt::from_bytes_be(
			Sign::Plus,
			&out_utxos[i].blinding.into_repr().to_bytes_be(),
		));
	}

	let inputs_for_proof = [
		("publicAmount", public_amount_as_vec.clone()),
		("extDataHash", ext_data_hash_as_vec.clone()),
		("inputNullifier", input_nullifier_as_vec.clone()),
		("inAmount", in_amount_as_vec.clone()),
		("inPrivateKey", in_private_key_as_vec.clone()),
		("inBlinding", in_blinding_as_vec.clone()),
		("inPathIndices", in_path_indices_as_vec.clone()),
		("inPathElements", in_path_elements_as_vec.clone()),
		("outputCommitment", output_commitment_as_vec.clone()),
		("outChainID", out_chain_id_as_vec.clone()),
		("outAmount", out_amount_as_vec.clone()),
		("outPubkey", out_pub_key_as_vec.clone()),
		("outBlinding", out_blinding_as_vec.clone()),
		("chainID", chain_id_as_vec.clone()),
		("roots", roots_as_vec.clone()),
	];

	let x = generate_proof(wc, circom_params, inputs_for_proof.clone());

	let num_inputs = circom_params.1.num_instance_variables;

	let (proof, full_assignment) = x.unwrap();

	let mut public_inputs = &full_assignment[1..num_inputs];

	(proof, public_inputs.to_vec())
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
	let chain_id_el = Element::from_bytes(&chain_id.into_repr().to_bytes_be());
	let public_amount_el = Element::from_bytes(&public_amount.into_repr().to_bytes_be());
	let root_set_el = roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let nullifiers_el = nullifiers
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let commitments_el = commitments
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_be());
	(chain_id_el, public_amount_el, root_set_el, nullifiers_el, commitments_el, ext_data_hash_el)
}

pub async fn expect_event<E: StaticEvent + Debug, T: Config, C: OnlineClientT<T>>(
	tx_progess: &mut TxProgress<T, C>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Start printing on a fresh line
	println!();

	while let Some(ev) = tx_progess.next_item().await {
		let ev: TxStatus<T, C> = ev?;

		match ev {
			Ready => {
				println!("Ready");
			},
			Broadcast(details) => {
				println!("Broadcasted: {details:?}");
			},
			InBlock(details) => {
				println!(
					"Transaction {:?} made it into block {:?}",
					details.extrinsic_hash(),
					details.block_hash()
				);

				let events = details.wait_for_success().await?;
				if let Some(event) = events.find_first::<E>()? {
					println!("In block (but not finalized): {event:?}");
				} else {
					println!("Failed to find Event");
				}
			},
			Finalized(details) => {
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
			},
			Future => {
				println!("Future");
			},
			Retracted(details) => {
				println!("Retracted: {details:?}");
			},
			FinalityTimeout(details) => {
				println!("FinalityTimeout: {details:?}");
			},
			Usurped(details) => {
				println!("Usurped: {details:?}");
			},
			Dropped => {
				println!("Dropped");
			},
			Invalid => {
				println!("Invalid");
			},
		}
	}

	Ok(())
}

pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
