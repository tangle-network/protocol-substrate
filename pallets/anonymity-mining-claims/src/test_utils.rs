use crate::mock::*;
use ark_bn254::{Bn254, Fr};
use ark_circom::{read_zkey, WitnessCalculator};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::ProvingKey;
use ark_relations::r1cs::ConstraintMatrices;
use ark_serialize::CanonicalSerialize;
use circom_proving::circom_from_folder;
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok};
use num_bigint::{BigInt, Sign};
use webb_primitives::{
	webb_proposals::{ResourceId, SubstrateTargetSystem, TargetSystem, TypedChainId},
	ElementTrait,
};

use std::{convert::TryInto, fs::File, str::FromStr, sync::Mutex};

use arkworks_setups::{common::setup_params, Curve};

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;

const SEED: u32 = 0;

pub fn setup_environment() {
	let curve = Curve::Bn254;
	let params3 = setup_params::<ark_bn254::Fr>(curve, 5, 3);

	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
	] {
		assert_ok!(Balances::set_balance(RuntimeOrigin::root(), account_id, 100_000_000, 0));
		assert_ok!(HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params3.to_bytes().try_into().unwrap(),
		));
	}
}
pub fn setup_environment_with_circom(
) -> ((ProvingKey<Bn254>, ConstraintMatrices<Fr>), &'static Mutex<WitnessCalculator>) {
	setup_environment();

	println!("Setting up ZKey");
	let path_2_2 = "../../solidity-fixtures/solidity-fixtures/reward_2/30/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	let wasm_2_2_path = "../../solidity-fixtures/solidity-fixtures/reward_2/30/reward_30_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

	println!("Setting up the verifier pallet");
	let mut vk_2_2_bytes = Vec::new();
	params_2_2.0.vk.serialize(&mut vk_2_2_bytes).unwrap();

	let param_call = ClaimsVerifier::force_set_parameters(
		RuntimeOrigin::root(),
		2,
		vk_2_2_bytes.try_into().unwrap(),
	);
	assert_ok!(param_call);

	(params_2_2, wc_2_2)
}
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
// #[serde(rename_all = "camelCase")]
pub struct InputsRaw {
	pub rate: String,
	pub fee: String,
	#[serde(rename = "rewardNullifier")]
	pub reward_nullifier: String,
	#[serde(rename = "extDataHash")]
	pub ext_data_hash: String,
	#[serde(rename = "noteChainID")]
	pub note_chain_id: String,
	#[serde(rename = "noteAmount")]
	pub note_amount: String,
	#[serde(rename = "noteAssetID")]
	pub note_asset_id: String,
	#[serde(rename = "noteTokenID")]
	pub note_token_id: String,
	#[serde(rename = "note_ak_X")]
	pub note_ak_x: String,
	#[serde(rename = "note_ak_Y")]
	pub note_ak_y: String,
	#[serde(rename = "noteBlinding")]
	pub note_blinding: String,
	#[serde(rename = "notePathElements")]
	pub note_path_elements: Vec<String>,
	#[serde(rename = "notePathIndices")]
	pub note_path_indices: String,
	pub note_alpha: String,
	#[serde(rename = "note_ak_alpha_X")]
	pub note_ak_alpha_x: String,
	#[serde(rename = "note_ak_alpha_Y")]
	pub note_ak_alpha_y: String,
	#[serde(rename = "inputChainID")]
	pub input_chain_id: String,
	#[serde(rename = "inputAmount")]
	pub input_amount: String,
	#[serde(rename = "inputPrivateKey")]
	pub input_private_key: String,
	#[serde(rename = "inputBlinding")]
	pub input_blinding: String,
	#[serde(rename = "inputNullifier")]
	pub input_nullifier: String,
	#[serde(rename = "inputRoot")]
	pub input_root: String,
	#[serde(rename = "inputPathElements")]
	pub input_path_elements: Vec<String>,
	#[serde(rename = "inputPathIndices")]
	pub input_path_indices: String,
	#[serde(rename = "outputChainID")]
	pub output_chain_id: String,
	#[serde(rename = "outputAmount")]
	pub output_amount: String,
	#[serde(rename = "outputPrivateKey")]
	pub output_private_key: String,
	#[serde(rename = "outputBlinding")]
	pub output_blinding: String,
	#[serde(rename = "outputCommitment")]
	pub output_commitment: String,
	#[serde(rename = "unspentTimestamp")]
	pub unspent_timestamp: String,
	#[serde(rename = "unspentRoots")]
	pub unspent_roots: Vec<String>,
	#[serde(rename = "unspentPathIndices")]
	pub unspent_path_indices: String,
	#[serde(rename = "unspentPathElements")]
	pub unspent_path_elements: Vec<String>,
	#[serde(rename = "spentTimestamp")]
	pub spent_timestamp: String,
	#[serde(rename = "spentRoots")]
	pub spent_roots: Vec<String>,
	#[serde(rename = "spentPathIndices")]
	pub spent_path_indices: String,
	#[serde(rename = "spentPathElements")]
	pub spent_path_elements: Vec<String>,
}

#[derive(Debug)]
pub struct RewardCircuitInputs {
	pub rate: Vec<BigInt>,
	pub fee: Vec<BigInt>,
	pub reward_nullifier: Vec<BigInt>,
	pub ext_data_hash: Vec<BigInt>,
	pub note_chain_id: Vec<BigInt>,
	pub note_amount: Vec<BigInt>,
	pub note_asset_id: Vec<BigInt>,
	pub note_token_id: Vec<BigInt>,
	pub note_ak_x: Vec<BigInt>,
	pub note_ak_y: Vec<BigInt>,
	pub note_blinding: Vec<BigInt>,
	pub note_path_elements: Vec<BigInt>,
	pub note_path_indices: Vec<BigInt>,
	pub note_alpha: Vec<BigInt>,
	pub note_ak_alpha_x: Vec<BigInt>,
	pub note_ak_alpha_y: Vec<BigInt>,
	pub input_chain_id: Vec<BigInt>,
	pub input_amount: Vec<BigInt>,
	pub input_private_key: Vec<BigInt>,
	pub input_blinding: Vec<BigInt>,
	pub input_nullifier: Vec<BigInt>,
	pub input_root: Vec<BigInt>,
	pub input_path_elements: Vec<BigInt>,
	pub input_path_indices: Vec<BigInt>,
	pub output_chain_id: Vec<BigInt>,
	pub output_amount: Vec<BigInt>,
	pub output_private_key: Vec<BigInt>,
	pub output_blinding: Vec<BigInt>,
	pub output_commitment: Vec<BigInt>,
	pub unspent_timestamp: Vec<BigInt>,
	pub unspent_roots: Vec<BigInt>,
	pub unspent_path_indices: Vec<BigInt>,
	pub unspent_path_elements: Vec<BigInt>,
	pub spent_timestamp: Vec<BigInt>,
	pub spent_roots: Vec<BigInt>,
	pub spent_path_indices: Vec<BigInt>,
	pub spent_path_elements: Vec<BigInt>,
}

impl RewardCircuitInputs {
	pub fn from_raw(inputs: &InputsRaw) -> Self {
		let rate = vec![Self::to_bigint(&inputs.rate)];
		let fee = vec![Self::to_bigint(&inputs.fee)];
		let reward_nullifier = vec![Self::to_bigint(&inputs.reward_nullifier)];
		let ext_data_hash = vec![Self::to_bigint(&inputs.ext_data_hash)];
		let note_chain_id = vec![Self::to_bigint(&inputs.note_chain_id)];
		let note_amount = vec![Self::to_bigint(&inputs.note_amount)];
		let note_asset_id = vec![Self::to_bigint(&inputs.note_asset_id)];
		let note_token_id = vec![Self::to_bigint(&inputs.note_token_id)];
		let note_ak_x = vec![Self::to_bigint(&inputs.note_ak_x)];
		let note_ak_y = vec![Self::to_bigint(&inputs.note_ak_y)];
		let note_blinding = vec![Self::to_bigint(&inputs.note_blinding)];
		let note_path_elements =
			inputs.note_path_elements.iter().map(|val| Self::to_bigint(val)).collect();
		let note_path_indices = vec![Self::to_bigint(&inputs.note_path_indices)];
		let note_alpha = vec![Self::to_bigint(&inputs.note_alpha)];
		let note_ak_alpha_x = vec![Self::to_bigint(&inputs.note_ak_alpha_x)];
		let note_ak_alpha_y = vec![Self::to_bigint(&inputs.note_ak_alpha_y)];
		let input_chain_id = vec![Self::to_bigint(&inputs.input_chain_id)];
		let input_amount = vec![Self::to_bigint(&inputs.input_amount)];
		let input_private_key = vec![Self::to_bigint(&inputs.input_private_key)];
		let input_blinding = vec![Self::to_bigint(&inputs.input_blinding)];
		let input_nullifier = vec![Self::to_bigint(&inputs.input_nullifier)];
		let input_root = vec![Self::to_bigint(&inputs.input_root)];
		let input_path_elements =
			inputs.input_path_elements.iter().map(|val| Self::to_bigint(val)).collect();

		let input_path_indices = vec![Self::to_bigint(&inputs.input_path_indices)];
		let output_chain_id = vec![Self::to_bigint(&inputs.output_chain_id)];
		let output_amount = vec![Self::to_bigint(&inputs.output_amount)];
		let output_private_key = vec![Self::to_bigint(&inputs.output_private_key)];
		let output_blinding = vec![Self::to_bigint(&inputs.output_blinding)];
		let output_commitment = vec![Self::to_bigint(&inputs.output_commitment)];
		let unspent_timestamp = vec![Self::to_bigint(&inputs.unspent_timestamp)];
		let unspent_roots = inputs.unspent_roots.iter().map(|root| Self::to_bigint(root)).collect();
		let unspent_path_indices = vec![Self::to_bigint(&inputs.unspent_path_indices)];
		let unspent_path_elements =
			inputs.unspent_path_elements.iter().map(|val| Self::to_bigint(val)).collect();
		let spent_timestamp = vec![Self::to_bigint(&inputs.spent_timestamp)];
		let spent_roots = inputs.spent_roots.iter().map(|val| Self::to_bigint(val)).collect();
		let spent_path_indices = vec![Self::to_bigint(&inputs.spent_path_indices)];
		let spent_path_elements =
			inputs.spent_path_elements.iter().map(|val| Self::to_bigint(val)).collect();
		Self {
			rate,
			fee,
			reward_nullifier,
			ext_data_hash,
			note_chain_id,
			note_amount,
			note_asset_id,
			note_token_id,
			note_ak_x,
			note_ak_y,
			note_blinding,
			note_path_elements,
			note_path_indices,
			note_alpha,
			note_ak_alpha_x,
			note_ak_alpha_y,
			input_chain_id,
			input_amount,
			input_private_key,
			input_blinding,
			input_nullifier,
			input_root,
			input_path_elements,
			input_path_indices,
			output_chain_id,
			output_amount,
			output_private_key,
			output_blinding,
			output_commitment,
			unspent_timestamp,
			unspent_roots,
			unspent_path_indices,
			unspent_path_elements,
			spent_timestamp,
			spent_roots,
			spent_path_indices,
			spent_path_elements,
		}
	}
	pub fn to_bigint(str_value: &str) -> BigInt {
		match str_value {
			hex_string if hex_string.starts_with("0x") =>
				BigInt::from_bytes_be(Sign::Plus, &hex::decode(&hex_string[2..]).unwrap()),
			decimal_string => BigInt::from_str(decimal_string).unwrap(),
		}
	}
}

pub fn deconstruct_public_inputs_reward_proof(
	max_edges: usize,
	public_inputs: &Vec<Bn254Fr>,
) -> (
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Vec<Bn254Fr>,
	Vec<Bn254Fr>,
) {
	let rate = public_inputs[0];
	let fee = public_inputs[1];
	let reward_nullifier = public_inputs[2];
	let note_ak_alpha_x = public_inputs[3];
	let note_ak_alpha_y = public_inputs[4];
	let ext_data_hash = public_inputs[5];
	let input_root = public_inputs[6];
	let input_nullifier = public_inputs[7];
	let output_commitment = public_inputs[8];
	let unspent_roots = public_inputs[9..9 + max_edges].to_vec();
	let spent_roots = public_inputs[9 + max_edges..9 + (2 * max_edges)].to_vec();
	(
		rate,
		fee,
		reward_nullifier,
		note_ak_alpha_x,
		note_ak_alpha_y,
		ext_data_hash,
		input_root,
		input_nullifier,
		output_commitment,
		unspent_roots,
		spent_roots,
	)
}

pub fn deconstruct_public_inputs_reward_proof_el(
	max_edges: u8,
	public_inputs_f: &Vec<Bn254Fr>,
) -> (
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Vec<Element>,
	Vec<Element>,
) {
	let (
		rate,
		fee,
		reward_nullifier,
		note_ak_alpha_x,
		note_ak_alpha_y,
		ext_data_hash,
		input_root,
		input_nullifier,
		output_commitment,
		spent_roots,
		unspent_roots,
	) = deconstruct_public_inputs_reward_proof(max_edges as usize, public_inputs_f);

	let rate_el = Element::from_bytes(&rate.into_repr().to_bytes_be());
	let fee_el = Element::from_bytes(&fee.into_repr().to_bytes_be());
	let _reward_nullifier_el = Element::from_bytes(&reward_nullifier.into_repr().to_bytes_be());
	let reward_nullifier_el = Element::from_bytes(&reward_nullifier.into_repr().to_bytes_be());
	let note_ak_alpha_x_el = Element::from_bytes(&note_ak_alpha_x.into_repr().to_bytes_be());
	let note_ak_alpha_y_el = Element::from_bytes(&note_ak_alpha_y.into_repr().to_bytes_be());
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_be());
	let input_root_el = Element::from_bytes(&input_root.into_repr().to_bytes_be());
	let input_nullifier_el = Element::from_bytes(&input_nullifier.into_repr().to_bytes_be());
	let output_commitment_el = Element::from_bytes(&output_commitment.into_repr().to_bytes_be());

	let unspent_roots_el = unspent_roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();

	let spent_roots_el = spent_roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();

	(
		rate_el,
		fee_el,
		reward_nullifier_el,
		note_ak_alpha_x_el,
		note_ak_alpha_y_el,
		ext_data_hash_el,
		input_root_el,
		input_nullifier_el,
		output_commitment_el,
		unspent_roots_el,
		spent_roots_el,
	)
}
