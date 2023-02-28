use crate::{
	mock::*,
	tests::*,
	// zerokit_utils::*,
	// Instance2,
};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_circom::{read_zkey, CircomConfig, CircomReduction, WitnessCalculator};
// use ark_ff::{BigInteger, BigInteger256, PrimeField, ToBytes};
use ark_groth16::{
	create_proof_with_reduction_and_matrices, create_random_proof as prove,
	generate_random_parameters, prepare_verifying_key, verify_proof as ark_verify_proof,
	Proof as ArkProof, ProvingKey, VerifyingKey,
};
use ark_relations::r1cs::{ConstraintMatrices, SynthesisError};
use ark_std::{rand::thread_rng, UniformRand};
use arkworks_native_gadgets::{
	merkle_tree::{Path, SparseMerkleTree},
	poseidon::Poseidon,
};
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path},
	utxo::Utxo,
	Curve,
};
use cfg_if::cfg_if;
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::OnInitialize};
use num_bigint::{BigInt, BigUint, Sign};
use once_cell::sync::OnceCell;
use pallet_linkable_tree::LinkableTreeConfigration;
use serde_json::Value;
use sp_core::hashing::keccak_256;
use std::{
	convert::{TryFrom, TryInto},
	fs::{self, File},
	io::{Cursor, Error, ErrorKind},
	result::Result,
	str::FromStr,
	sync::Mutex,
};
use thiserror::Error;
use wasmer::{Module, Store};
use webb_primitives::{
	linkable_tree::LinkableTreeInspector, merkle_tree::TreeInspector, utils::compute_chain_id_type,
	verifying::CircomError, AccountId,
};

type Bn254Fr = ark_bn254::Fr;

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
	vanchor_witness: [(&str, Vec<BigInt>); 37],
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
	inputs: Vec<Fr>,
) -> Result<bool, ProofError> {
	// Check that the proof is valid
	let pvk = prepare_verifying_key(verifying_key);
	//let pr: ArkProof<Curve> = (*proof).into();

	let verified = ark_verify_proof(&pvk, proof, &inputs)?;

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

fn setup_environment_with_circom(
) -> ((ProvingKey<Bn254>, ConstraintMatrices<Fr>), &'static Mutex<WitnessCalculator>) {
	let curve = Curve::Bn254;
	let params3 = setup_params::<ark_bn254::Fr>(curve, 5, 3);

	println!("Setting up ZKey");
	let path_2_2 = "/home/semar/Projects/protocol-substrate/pallets/anonymity-mining-rewards/solidity-fixtures/solidity-fixtures/reward_2/30/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	let wasm_2_2_path = "/home/semar/Projects/protocol-substrate/pallets/anonymity-mining-rewards/solidity-fixtures/solidity-fixtures/reward_2/30/reward_30_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

	// let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	// let relayer = account::<AccountId>("", RELAYER_ACCOUNT_ID, SEED);
	//
	// // Set balances
	// assert_ok!(Balances::set_balance(RuntimeOrigin::root(), transactor, DEFAULT_BALANCE, 0));
	// assert_ok!(Balances::set_balance(RuntimeOrigin::root(), relayer, DEFAULT_BALANCE, 0));

	// finally return the provingkey bytes
	(params_2_2, wc_2_2)
}

// #[test]
// fn circom_should_complete_30x2_reward_claim_with_fixed_values() {
// 	new_test_ext().execute_with(|| {
// 		let params4 = setup_params::<Bn254Fr>(Curve::Bn254, 5, 4);
// 		let nullifier_hasher = Poseidon::<Bn254Fr> { params: params4 };
// 		let (params_2_2, wc_2_2) = setup_environment_with_circom();
// 		let tree_id = MerkleTree2::next_tree_id() - 1;
//
// 		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
// 		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
// 		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
//
// 		let ext_amount: Amount = 10_i128;
// 		let public_amount = 10_i128;
// 		let fee: Balance = 0;
//
// 		let chain_type = [2, 0];
// 		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
// 		let in_chain_ids = [chain_id; 2];
// 		let in_amounts = [0, 0];
// 		let in_indices = [0, 1];
// 		let out_chain_ids = [chain_id; 2];
// 		let out_amounts = [10, 0];
//
// 		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
// 		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);
//
// 		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
// 		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
// 		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
// 			recipient.clone(),
// 			relayer.clone(),
// 			ext_amount,
// 			fee,
// 			0,
// 			0,
// 			// Mock encryption value, not meant to be used in production
// 			output1.to_vec(),
// 			// Mock encryption value, not meant to be used in production
// 			output2.to_vec(),
// 		);
// 		println!("ext_data: {:?}", ext_data);
// 		let ext_data_hash = keccak_256(&ext_data.encode_abi());
//
// 		let custom_root = MerkleTree2::get_default_root(tree_id).unwrap();
// 		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree2 as LinkableTreeInspector<
// 			LinkableTreeConfigration<Test, Instance2>,
// 		>>::get_neighbor_roots(tree_id)
// 		.unwrap()
// 		.try_into()
// 		.unwrap();
// 		println!("neighbor_roots: {:?}", neighbor_roots);
//
// 		let input_nullifiers = in_utxos
// 			.clone()
// 			.map(|utxo| utxo.calculate_nullifier(&nullifier_hasher).unwrap());
//
// 		let (in_indices, _in_root_set, _tree, in_paths) =
// 			insert_utxos_to_merkle_tree(&in_utxos, neighbor_roots, custom_root);
//
// 		// Make Inputs
// 		let public_amount = if public_amount > 0 {
// 			vec![BigInt::from_bytes_be(Sign::Plus, &public_amount.to_be_bytes())]
// 		} else {
// 			vec![BigInt::from_bytes_be(Sign::Minus, &(-public_amount).to_be_bytes())]
// 		};
//
// 		let mut ext_data_hash = public_amount.clone();
// 		let mut input_nullifier = Vec::new();
// 		let mut output_commitment = Vec::new();
// 		for i in 0..NUM_UTXOS {
// 			input_nullifier.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&input_nullifiers[i].into_repr().to_bytes_be(),
// 			));
// 			output_commitment.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&out_utxos[i].commitment.into_repr().to_bytes_be(),
// 			));
// 		}
//
// 		let mut chain_id = vec![BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes())];
//
// 		let mut roots = Vec::new();
//
// 		roots.push(BigInt::from_bytes_be(Sign::Plus, &custom_root.0));
// 		for i in 0..ANCHOR_CT - 1 {
// 			roots.push(BigInt::from_bytes_be(Sign::Plus, &neighbor_roots[i].0));
// 		}
//
// 		let mut in_amount = Vec::new();
// 		let mut in_private_key = Vec::new();
// 		let mut in_blinding = Vec::new();
// 		let mut in_path_indices = Vec::new();
// 		let mut in_path_elements = Vec::new();
// 		let mut out_chain_id = Vec::new();
// 		let mut out_amount = Vec::new();
// 		let mut out_pub_key = Vec::new();
// 		let mut out_blinding = Vec::new();
//
// 		for i in 0..NUM_UTXOS {
// 			in_amount.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&in_utxos[i].amount.into_repr().to_bytes_be(),
// 			));
// 			in_private_key.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&in_utxos[i].keypair.secret_key.unwrap().into_repr().to_bytes_be(),
// 			));
// 			in_blinding.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&in_utxos[i].blinding.into_repr().to_bytes_be(),
// 			));
// 			in_path_indices.push(BigInt::from(in_indices[i]));
// 			for j in 0..TREE_DEPTH {
// 				let neighbor_elt: Bn254Fr =
// 					if in_indices[i] == 0 { in_paths[i].path[j].1 } else { in_paths[i].path[j].0 };
// 				in_path_elements.push(BigInt::from_bytes_be(
// 					Sign::Plus,
// 					&neighbor_elt.into_repr().to_bytes_be(),
// 				));
// 			}
//
// 			out_chain_id.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&out_utxos[i].chain_id.into_repr().to_bytes_be(),
// 			));
//
// 			out_amount.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&out_utxos[i].amount.into_repr().to_bytes_be(),
// 			));
//
// 			out_pub_key.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&out_utxos[i].keypair.public_key.into_repr().to_bytes_be(),
// 			));
//
// 			out_blinding.push(BigInt::from_bytes_be(
// 				Sign::Plus,
// 				&out_utxos[i].blinding.into_repr().to_bytes_be(),
// 			));
// 		}
//
// 		let inputs_for_proof = [
// 			("publicAmount", public_amount.clone()),
// 			("extDataHash", ext_data_hash.clone()),
// 			("inputNullifier", input_nullifier.clone()),
// 			("inAmount", in_amount.clone()),
// 			("inPrivateKey", in_private_key.clone()),
// 			("inBlinding", in_blinding.clone()),
// 			("inPathIndices", in_path_indices.clone()),
// 			("inPathElements", in_path_elements.clone()),
// 			("outputCommitment", output_commitment.clone()),
// 			("outChainID", out_chain_id.clone()),
// 			("outAmount", out_amount.clone()),
// 			("outPubkey", out_pub_key.clone()),
// 			("outBlinding", out_blinding.clone()),
// 			("chainID", chain_id.clone()),
// 			("roots", roots.clone()),
// 		];
//
// 		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());
//
// 		let num_inputs = params_2_2.1.num_instance_variables;
//
// 		let (proof, full_assignment) = x.unwrap();
//
// 		let mut inputs_for_verification = &full_assignment[1..num_inputs];
//
// 		println!(
// 			"v {:?} {:?}",
// 			inputs_for_verification.len(),
// 			inputs_for_verification
// 				.into_iter()
// 				.map(|x| to_bigint(&x))
// 				.collect::<Vec<BigInt>>()
// 		);
//
// 		let did_proof_work =
// 			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
// 		assert!(did_proof_work);
// 	});
// }
//
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
// #[serde(rename_all = "camelCase")]
struct InputsRaw {
	rate: String,
	fee: String,
	#[serde(rename = "rewardNullifier")]
	reward_nullifier: String,
	#[serde(rename = "extDataHash")]
	ext_data_hash: String,
	#[serde(rename = "noteChainID")]
	note_chain_id: String,
	#[serde(rename = "noteAmount")]
	note_amount: String,
	#[serde(rename = "noteAssetID")]
	note_asset_id: String,
	#[serde(rename = "noteTokenID")]
	note_token_id: String,
	#[serde(rename = "note_ak_X")]
	note_ak_x: String,
	#[serde(rename = "note_ak_Y")]
	note_ak_y: String,
	#[serde(rename = "noteBlinding")]
	note_blinding: String,
	#[serde(rename = "notePathElements")]
	note_path_elements: Vec<String>,
	#[serde(rename = "notePathIndices")]
	note_path_indices: String,
	note_alpha: String,
	#[serde(rename = "note_ak_alpha_X")]
	note_ak_alpha_x: String,
	#[serde(rename = "note_ak_alpha_Y")]
	note_ak_alpha_y: String,
	#[serde(rename = "inputChainID")]
	input_chain_id: String,
	#[serde(rename = "inputAmount")]
	input_amount: String,
	#[serde(rename = "inputPrivateKey")]
	input_private_key: String,
	#[serde(rename = "inputBlinding")]
	input_blinding: String,
	#[serde(rename = "inputNullifier")]
	input_nullifier: String,
	#[serde(rename = "inputRoot")]
	input_root: String,
	#[serde(rename = "inputPathElements")]
	input_path_elements: Vec<String>,
	#[serde(rename = "inputPathIndices")]
	input_path_indices: String,
	#[serde(rename = "outputChainID")]
	output_chain_id: String,
	#[serde(rename = "outputAmount")]
	output_amount: String,
	#[serde(rename = "outputPrivateKey")]
	output_private_key: String,
	#[serde(rename = "outputBlinding")]
	output_blinding: String,
	#[serde(rename = "outputCommitment")]
	output_commitment: String,
	#[serde(rename = "unspentTimestamp")]
	unspent_timestamp: String,
	#[serde(rename = "unspentRoots")]
	unspent_roots: Vec<String>,
	#[serde(rename = "unspentPathIndices")]
	unspent_path_indices: String,
	#[serde(rename = "unspentPathElements")]
	unspent_path_elements: Vec<String>,
	#[serde(rename = "spentTimestamp")]
	spent_timestamp: String,
	#[serde(rename = "spentRoots")]
	spent_roots: Vec<String>,
	#[serde(rename = "spentPathIndices")]
	spent_path_indices: String,
	#[serde(rename = "spentPathElements")]
	spent_path_elements: Vec<String>,
}

#[derive(Debug)]
struct RewardCircuitInputs {
	rate: Vec<BigInt>,
	fee: Vec<BigInt>,
	reward_nullifier: Vec<BigInt>,
	ext_data_hash: Vec<BigInt>,
	note_chain_id: Vec<BigInt>,
	note_amount: Vec<BigInt>,
	note_asset_id: Vec<BigInt>,
	note_token_id: Vec<BigInt>,
	note_ak_x: Vec<BigInt>,
	note_ak_y: Vec<BigInt>,
	note_blinding: Vec<BigInt>,
	note_path_elements: Vec<BigInt>,
	note_path_indices: Vec<BigInt>,
	note_alpha: Vec<BigInt>,
	note_ak_alpha_x: Vec<BigInt>,
	note_ak_alpha_y: Vec<BigInt>,
	input_chain_id: Vec<BigInt>,
	input_amount: Vec<BigInt>,
	input_private_key: Vec<BigInt>,
	input_blinding: Vec<BigInt>,
	input_nullifier: Vec<BigInt>,
	input_root: Vec<BigInt>,
	input_path_elements: Vec<BigInt>,
	input_path_indices: Vec<BigInt>,
	output_chain_id: Vec<BigInt>,
	output_amount: Vec<BigInt>,
	output_private_key: Vec<BigInt>,
	output_blinding: Vec<BigInt>,
	output_commitment: Vec<BigInt>,
	unspent_timestamp: Vec<BigInt>,
	unspent_roots: Vec<BigInt>,
	unspent_path_indices: Vec<BigInt>,
	unspent_path_elements: Vec<BigInt>,
	spent_timestamp: Vec<BigInt>,
	spent_roots: Vec<BigInt>,
	spent_path_indices: Vec<BigInt>,
	spent_path_elements: Vec<BigInt>,
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
		let note_path_elements = inputs.note_path_elements.iter().map(|val| Self::to_bigint(&val)).collect();
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
		let input_path_elements = inputs.input_path_elements.iter().map(|val| Self::to_bigint(&val)).collect();

		let input_path_indices = vec![Self::to_bigint(&inputs.input_path_indices)];
		let output_chain_id = vec![Self::to_bigint(&inputs.output_chain_id)];
		let output_amount = vec![Self::to_bigint(&inputs.output_amount)];
		let output_private_key = vec![Self::to_bigint(&inputs.output_private_key)];
		let output_blinding = vec![Self::to_bigint(&inputs.output_blinding)];
		let output_commitment = vec![Self::to_bigint(&inputs.output_commitment)];
		let unspent_timestamp = vec![Self::to_bigint(&inputs.unspent_timestamp)];
		let unspent_roots = inputs.unspent_roots.iter().map(|root|  Self::to_bigint(&root)).collect();
		let unspent_path_indices = vec![Self::to_bigint(&inputs.unspent_path_indices)];
		let unspent_path_elements = inputs.unspent_path_elements.iter().map(|val| Self::to_bigint(&val)).collect();
		let spent_timestamp = vec![Self::to_bigint(&inputs.spent_timestamp)];
		let spent_roots = inputs.spent_roots.iter().map(|val| Self::to_bigint(&val)).collect();
		let spent_path_indices = vec![Self::to_bigint(&inputs.spent_path_indices)];
		let spent_path_elements = inputs.spent_path_elements.iter().map(|val| Self::to_bigint(&val)).collect();
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
	fn to_bigint(str_value: &str) -> BigInt {
		match str_value {
			hex_string if hex_string.starts_with("0x") =>
				BigInt::from_bytes_be(Sign::Plus, &hex::decode(&hex_string[2..]).unwrap()),
			decimal_string => BigInt::from_str(decimal_string).unwrap(),
		}
	}
}

#[test]
fn circom_should_complete_30x2_reward_claim_with_json_file() {
	new_test_ext().execute_with(|| {
		let (params_2_2, wc_2_2) = setup_environment_with_circom();
		let raw = include_str!("../circuitInput.json");
		let inputs_raw: InputsRaw = serde_json::from_str(raw).unwrap();
		let circuit_inputs: RewardCircuitInputs = RewardCircuitInputs::from_raw(&inputs_raw);
		// println!("inputs: {inputs_raw:?}");
		println!("circuitInputs: {circuit_inputs:?}");

		let inputs_for_proof = [
			("rate", circuit_inputs.rate.clone()),
			("fee", circuit_inputs.fee.clone()),
			("rewardNullifier", circuit_inputs.reward_nullifier.clone()),
			("extDataHash", circuit_inputs.ext_data_hash.clone()),
			("noteChainID", circuit_inputs.note_chain_id.clone()),
			("noteAmount", circuit_inputs.note_amount.clone()),
			("noteAssetID", circuit_inputs.note_asset_id.clone()),
			("noteTokenID", circuit_inputs.note_token_id.clone()),
			("note_ak_X", circuit_inputs.note_ak_x.clone()),
			("note_ak_Y", circuit_inputs.note_ak_y.clone()),
			("noteBlinding", circuit_inputs.note_blinding.clone()),
			("notePathElements", circuit_inputs.note_path_elements.clone()),
			("notePathIndices", circuit_inputs.note_path_indices.clone()),
			("note_alpha", circuit_inputs.note_alpha.clone()),
			("note_ak_alpha_X", circuit_inputs.note_ak_alpha_x.clone()),
			("note_ak_alpha_Y", circuit_inputs.note_ak_alpha_y.clone()),
			("inputChainID", circuit_inputs.input_chain_id.clone()),
			("inputAmount", circuit_inputs.input_amount.clone()),
			("inputPrivateKey", circuit_inputs.input_private_key.clone()),
			("inputBlinding", circuit_inputs.input_blinding.clone()),
			("inputNullifier", circuit_inputs.input_nullifier.clone()),
			("inputRoot", circuit_inputs.input_root.clone()),
			("inputPathElements", circuit_inputs.input_path_elements.clone()),
			("inputPathIndices", circuit_inputs.input_path_indices.clone()),
			("outputChainID", circuit_inputs.output_chain_id.clone()),
			("outputAmount", circuit_inputs.output_amount.clone()),
			("outputPrivateKey", circuit_inputs.output_private_key.clone()),
			("outputBlinding", circuit_inputs.output_blinding.clone()),
			("outputCommitment", circuit_inputs.output_commitment.clone()),
			("unspentTimestamp", circuit_inputs.unspent_timestamp.clone()),
			("unspentRoots", circuit_inputs.unspent_roots.clone()),
			("unspentPathIndices", circuit_inputs.unspent_path_indices.clone()),
			("unspentPathElements", circuit_inputs.unspent_path_elements.clone()),
			("spentTimestamp", circuit_inputs.spent_timestamp.clone()),
			("spentRoots", circuit_inputs.spent_roots.clone()),
			("spentPathIndices", circuit_inputs.spent_path_indices.clone()),
			("spentPathElements", circuit_inputs.spent_path_elements.clone()),
		];
		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let mut inputs_for_verification = &full_assignment[1..num_inputs];

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);
	});
}
#[test]
fn circom_should_complete_30x2_reward_claim_with_fixed_values() {
	new_test_ext().execute_with(|| {
		let (params_2_2, wc_2_2) = setup_environment_with_circom();

		let rate = vec![BigInt::from_str("1000").unwrap()];
		let fee = vec![BigInt::from_str("0").unwrap()];
		let reward_nullifier = vec![BigInt::from_str(
			"10564965585879388820704898650847204303100381631451275255108222499052368642090",
		)
		.unwrap()];
		let ext_data_hash = vec![BigInt::from_str(
			"358126935078995354426013562884265319790841558842708315026705324586616062935",
		)
		.unwrap()];
		let note_chain_id = vec![BigInt::from_str("1099511659113").unwrap()];
		let note_amount = vec![BigInt::from_str("10000000").unwrap()];
		let note_asset_id = vec![BigInt::from_str("1").unwrap()];
		let note_token_id = vec![BigInt::from_str("0").unwrap()];
		let note_ak_x = vec![BigInt::from_str(
			"9362477857622610446419033445002439620323338088976740506400769274235587248536",
		)
		.unwrap()];
		let note_ak_y = vec![BigInt::from_str(
			"9691881433226105758118067955392105848256146540575750315666276637207068615290",
		)
		.unwrap()];
		let note_blinding = vec![BigInt::from_str(
			"302477276991354938872515923916066470619268911752558013427752741286619214978",
		)
		.unwrap()];
		let note_path_elements = vec![
			BigInt::from_str(
				"21663839004416932945382355908790599225266501822907911457504978515578255421292",
			)
			.unwrap(),
			BigInt::from_str(
				"8995896153219992062710898675021891003404871425075198597897889079729967997688",
			)
			.unwrap(),
			BigInt::from_str(
				"15126246733515326086631621937388047923581111613947275249184377560170833782629",
			)
			.unwrap(),
			BigInt::from_str(
				"6404200169958188928270149728908101781856690902670925316782889389790091378414",
			)
			.unwrap(),
			BigInt::from_str(
				"17903822129909817717122288064678017104411031693253675943446999432073303897479",
			)
			.unwrap(),
			BigInt::from_str(
				"11423673436710698439362231088473903829893023095386581732682931796661338615804",
			)
			.unwrap(),
			BigInt::from_str(
				"10494842461667482273766668782207799332467432901404302674544629280016211342367",
			)
			.unwrap(),
			BigInt::from_str(
				"17400501067905286947724900644309270241576392716005448085614420258732805558809",
			)
			.unwrap(),
			BigInt::from_str(
				"7924095784194248701091699324325620647610183513781643345297447650838438175245",
			)
			.unwrap(),
			BigInt::from_str(
				"3170907381568164996048434627595073437765146540390351066869729445199396390350",
			)
			.unwrap(),
			BigInt::from_str(
				"21224698076141654110749227566074000819685780865045032659353546489395159395031",
			)
			.unwrap(),
			BigInt::from_str(
				"18113275293366123216771546175954550524914431153457717566389477633419482708807",
			)
			.unwrap(),
			BigInt::from_str(
				"1952712013602708178570747052202251655221844679392349715649271315658568301659",
			)
			.unwrap(),
			BigInt::from_str(
				"18071586466641072671725723167170872238457150900980957071031663421538421560166",
			)
			.unwrap(),
			BigInt::from_str(
				"9993139859464142980356243228522899168680191731482953959604385644693217291503",
			)
			.unwrap(),
			BigInt::from_str(
				"14825089209834329031146290681677780462512538924857394026404638992248153156554",
			)
			.unwrap(),
			BigInt::from_str(
				"4227387664466178643628175945231814400524887119677268757709033164980107894508",
			)
			.unwrap(),
			BigInt::from_str(
				"177945332589823419436506514313470826662740485666603469953512016396504401819",
			)
			.unwrap(),
			BigInt::from_str(
				"4236715569920417171293504597566056255435509785944924295068274306682611080863",
			)
			.unwrap(),
			BigInt::from_str(
				"8055374341341620501424923482910636721817757020788836089492629714380498049891",
			)
			.unwrap(),
			BigInt::from_str(
				"19476726467694243150694636071195943429153087843379888650723427850220480216251",
			)
			.unwrap(),
			BigInt::from_str(
				"10399152597354305597912907824309239409817888262144718512380299069916269589113",
			)
			.unwrap(),
			BigInt::from_str(
				"15258214568755945879473165452099056399152522811554450867424704782105583612159",
			)
			.unwrap(),
			BigInt::from_str(
				"17875137458200521792878123299688785698619987766564867631132505563522438895970",
			)
			.unwrap(),
			BigInt::from_str(
				"10256337741884550510386441312852347693461186988936458078715216316630265625616",
			)
			.unwrap(),
			BigInt::from_str(
				"15901668779096826777384382311437482837232013711720349479070803061728347594771",
			)
			.unwrap(),
			BigInt::from_str(
				"20742074412337132804130107876526602108986687824137082336286554983131679140884",
			)
			.unwrap(),
			BigInt::from_str(
				"486721614616614661899683022609286155116112838076254734405175415967221587136",
			)
			.unwrap(),
			BigInt::from_str(
				"14553704920999879116288394486755121409791035465066405350421564694668688837968",
			)
			.unwrap(),
			BigInt::from_str(
				"16133427046514840932414333181474318783048630149216801191847554534526344556935",
			)
			.unwrap(),
		];
		let note_path_indices = vec![BigInt::from_str("0").unwrap()];
		let note_alpha = vec![BigInt::from_str(
			"6123037454567157330841838886359697856896282432923527003048763473381774494143",
		)
		.unwrap()];
		let note_ak_alpha_x = vec![BigInt::from_str(
			"16790413150265921314496255255442048864691300484387763290323717792276398806784",
		)
		.unwrap()];
		let note_ak_alpha_y = vec![BigInt::from_str(
			"18033778599169003681549362690976134187951785981919739369035190304149050830164",
		)
		.unwrap()];
		let input_chain_id = vec![BigInt::from_str("1099511659113").unwrap()];
		let input_amount = vec![BigInt::from_str("0").unwrap()];
		let input_private_key = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("2df7c52e1a02052134f20c87cc4050e6a9db6b21fdf0fb9d02d7e12557c359c4")
				.unwrap(),
		)];
		let input_blinding = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("e2d644eb7a6239fdb9bf0f92d19a4780758ed21de2ebe96868adfeed9afb11").unwrap(),
		)];
		let input_nullifier = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("17e2ce1c4ed5f14870b450c8da6337132ba64853fb3e65249cd10be40a53c545")
				.unwrap(),
		)];
		let input_root = vec![BigInt::from_str(
			"14059412023772412256724985181143164416212338358739056294444144007074935376544",
		)
		.unwrap()];
		let input_path_elements = vec![
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
			BigInt::from(0),
		];

		let input_path_indices = vec![BigInt::from_str("0").unwrap()];
		let output_chain_id = vec![BigInt::from_str("1099511659113").unwrap()];
		let output_amount = vec![BigInt::from_str("10060000000000").unwrap()];
		let output_private_key = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("11cadb169d96bae2856478ed12e5ae2b5424e5f4118b7571e916a72718f133cc")
				.unwrap(),
		)];
		let output_blinding = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("fd48f427fc60838a5114cf67c6e169c50805ee698d66e8969d41a0a0d1a4cc").unwrap(),
		)];
		let output_commitment = vec![BigInt::from_bytes_be(
			Sign::Plus,
			&hex::decode("1c12edcd46b10bce2d9be9954a667f9b6094a47ff55cd02a7bd69b3a00c12890")
				.unwrap(),
		)];
		let unspent_timestamp = vec![BigInt::from_str("1677530326491").unwrap()];
		let unspent_roots = vec![
			BigInt::from_str(
				"7113069105662469755354285165943659123950091801355230016253064568711890313563",
			)
			.unwrap(),
			BigInt::from_str(
				"14059412023772412256724985181143164416212338358739056294444144007074935376544",
			)
			.unwrap(),
		];
		let unspent_path_indices = vec![BigInt::from_str("0").unwrap()];
		let unspent_path_elements = vec![
			BigInt::from_str(
				"21663839004416932945382355908790599225266501822907911457504978515578255421292",
			)
			.unwrap(),
			BigInt::from_str(
				"8995896153219992062710898675021891003404871425075198597897889079729967997688",
			)
			.unwrap(),
			BigInt::from_str(
				"15126246733515326086631621937388047923581111613947275249184377560170833782629",
			)
			.unwrap(),
			BigInt::from_str(
				"6404200169958188928270149728908101781856690902670925316782889389790091378414",
			)
			.unwrap(),
			BigInt::from_str(
				"17903822129909817717122288064678017104411031693253675943446999432073303897479",
			)
			.unwrap(),
			BigInt::from_str(
				"11423673436710698439362231088473903829893023095386581732682931796661338615804",
			)
			.unwrap(),
			BigInt::from_str(
				"10494842461667482273766668782207799332467432901404302674544629280016211342367",
			)
			.unwrap(),
			BigInt::from_str(
				"17400501067905286947724900644309270241576392716005448085614420258732805558809",
			)
			.unwrap(),
			BigInt::from_str(
				"7924095784194248701091699324325620647610183513781643345297447650838438175245",
			)
			.unwrap(),
			BigInt::from_str(
				"3170907381568164996048434627595073437765146540390351066869729445199396390350",
			)
			.unwrap(),
			BigInt::from_str(
				"21224698076141654110749227566074000819685780865045032659353546489395159395031",
			)
			.unwrap(),
			BigInt::from_str(
				"18113275293366123216771546175954550524914431153457717566389477633419482708807",
			)
			.unwrap(),
			BigInt::from_str(
				"1952712013602708178570747052202251655221844679392349715649271315658568301659",
			)
			.unwrap(),
			BigInt::from_str(
				"18071586466641072671725723167170872238457150900980957071031663421538421560166",
			)
			.unwrap(),
			BigInt::from_str(
				"9993139859464142980356243228522899168680191731482953959604385644693217291503",
			)
			.unwrap(),
			BigInt::from_str(
				"14825089209834329031146290681677780462512538924857394026404638992248153156554",
			)
			.unwrap(),
			BigInt::from_str(
				"4227387664466178643628175945231814400524887119677268757709033164980107894508",
			)
			.unwrap(),
			BigInt::from_str(
				"177945332589823419436506514313470826662740485666603469953512016396504401819",
			)
			.unwrap(),
			BigInt::from_str(
				"4236715569920417171293504597566056255435509785944924295068274306682611080863",
			)
			.unwrap(),
			BigInt::from_str(
				"8055374341341620501424923482910636721817757020788836089492629714380498049891",
			)
			.unwrap(),
			BigInt::from_str(
				"19476726467694243150694636071195943429153087843379888650723427850220480216251",
			)
			.unwrap(),
			BigInt::from_str(
				"10399152597354305597912907824309239409817888262144718512380299069916269589113",
			)
			.unwrap(),
			BigInt::from_str(
				"15258214568755945879473165452099056399152522811554450867424704782105583612159",
			)
			.unwrap(),
			BigInt::from_str(
				"17875137458200521792878123299688785698619987766564867631132505563522438895970",
			)
			.unwrap(),
			BigInt::from_str(
				"10256337741884550510386441312852347693461186988936458078715216316630265625616",
			)
			.unwrap(),
			BigInt::from_str(
				"15901668779096826777384382311437482837232013711720349479070803061728347594771",
			)
			.unwrap(),
			BigInt::from_str(
				"20742074412337132804130107876526602108986687824137082336286554983131679140884",
			)
			.unwrap(),
			BigInt::from_str(
				"486721614616614661899683022609286155116112838076254734405175415967221587136",
			)
			.unwrap(),
			BigInt::from_str(
				"14553704920999879116288394486755121409791035465066405350421564694668688837968",
			)
			.unwrap(),
			BigInt::from_str(
				"16133427046514840932414333181474318783048630149216801191847554534526344556935",
			)
			.unwrap(),
		];
		let spent_timestamp = vec![BigInt::from_str("1677530327497").unwrap()];
		let spent_roots = vec![
			BigInt::from_str(
				"4604994298768447405083279630799635882620876532350700039390340817159683796313",
			)
			.unwrap(),
			BigInt::from_str(
				"14059412023772412256724985181143164416212338358739056294444144007074935376544",
			)
			.unwrap(),
		];
		let spent_path_indices = vec![BigInt::from_str("0").unwrap()];
		let spent_path_elements = vec![
			BigInt::from_str(
				"21663839004416932945382355908790599225266501822907911457504978515578255421292",
			)
			.unwrap(),
			BigInt::from_str(
				"8995896153219992062710898675021891003404871425075198597897889079729967997688",
			)
			.unwrap(),
			BigInt::from_str(
				"15126246733515326086631621937388047923581111613947275249184377560170833782629",
			)
			.unwrap(),
			BigInt::from_str(
				"6404200169958188928270149728908101781856690902670925316782889389790091378414",
			)
			.unwrap(),
			BigInt::from_str(
				"17903822129909817717122288064678017104411031693253675943446999432073303897479",
			)
			.unwrap(),
			BigInt::from_str(
				"11423673436710698439362231088473903829893023095386581732682931796661338615804",
			)
			.unwrap(),
			BigInt::from_str(
				"10494842461667482273766668782207799332467432901404302674544629280016211342367",
			)
			.unwrap(),
			BigInt::from_str(
				"17400501067905286947724900644309270241576392716005448085614420258732805558809",
			)
			.unwrap(),
			BigInt::from_str(
				"7924095784194248701091699324325620647610183513781643345297447650838438175245",
			)
			.unwrap(),
			BigInt::from_str(
				"3170907381568164996048434627595073437765146540390351066869729445199396390350",
			)
			.unwrap(),
			BigInt::from_str(
				"21224698076141654110749227566074000819685780865045032659353546489395159395031",
			)
			.unwrap(),
			BigInt::from_str(
				"18113275293366123216771546175954550524914431153457717566389477633419482708807",
			)
			.unwrap(),
			BigInt::from_str(
				"1952712013602708178570747052202251655221844679392349715649271315658568301659",
			)
			.unwrap(),
			BigInt::from_str(
				"18071586466641072671725723167170872238457150900980957071031663421538421560166",
			)
			.unwrap(),
			BigInt::from_str(
				"9993139859464142980356243228522899168680191731482953959604385644693217291503",
			)
			.unwrap(),
			BigInt::from_str(
				"14825089209834329031146290681677780462512538924857394026404638992248153156554",
			)
			.unwrap(),
			BigInt::from_str(
				"4227387664466178643628175945231814400524887119677268757709033164980107894508",
			)
			.unwrap(),
			BigInt::from_str(
				"177945332589823419436506514313470826662740485666603469953512016396504401819",
			)
			.unwrap(),
			BigInt::from_str(
				"4236715569920417171293504597566056255435509785944924295068274306682611080863",
			)
			.unwrap(),
			BigInt::from_str(
				"8055374341341620501424923482910636721817757020788836089492629714380498049891",
			)
			.unwrap(),
			BigInt::from_str(
				"19476726467694243150694636071195943429153087843379888650723427850220480216251",
			)
			.unwrap(),
			BigInt::from_str(
				"10399152597354305597912907824309239409817888262144718512380299069916269589113",
			)
			.unwrap(),
			BigInt::from_str(
				"15258214568755945879473165452099056399152522811554450867424704782105583612159",
			)
			.unwrap(),
			BigInt::from_str(
				"17875137458200521792878123299688785698619987766564867631132505563522438895970",
			)
			.unwrap(),
			BigInt::from_str(
				"10256337741884550510386441312852347693461186988936458078715216316630265625616",
			)
			.unwrap(),
			BigInt::from_str(
				"15901668779096826777384382311437482837232013711720349479070803061728347594771",
			)
			.unwrap(),
			BigInt::from_str(
				"20742074412337132804130107876526602108986687824137082336286554983131679140884",
			)
			.unwrap(),
			BigInt::from_str(
				"486721614616614661899683022609286155116112838076254734405175415967221587136",
			)
			.unwrap(),
			BigInt::from_str(
				"14553704920999879116288394486755121409791035465066405350421564694668688837968",
			)
			.unwrap(),
			BigInt::from_str(
				"16133427046514840932414333181474318783048630149216801191847554534526344556935",
			)
			.unwrap(),
		];
		let inputs_for_proof = [
			("rate", rate.clone()),
			("fee", fee.clone()),
			("rewardNullifier", reward_nullifier.clone()),
			("extDataHash", ext_data_hash.clone()),
			("noteChainID", note_chain_id.clone()),
			("noteAmount", note_amount.clone()),
			("noteAssetID", note_asset_id.clone()),
			("noteTokenID", note_token_id.clone()),
			("note_ak_X", note_ak_x.clone()),
			("note_ak_Y", note_ak_y.clone()),
			("noteBlinding", note_blinding.clone()),
			("notePathElements", note_path_elements.clone()),
			("notePathIndices", note_path_indices.clone()),
			("note_alpha", note_alpha.clone()),
			("note_ak_alpha_X", note_ak_alpha_x.clone()),
			("note_ak_alpha_Y", note_ak_alpha_y.clone()),
			("inputChainID", input_chain_id.clone()),
			("inputAmount", input_amount.clone()),
			("inputPrivateKey", input_private_key.clone()),
			("inputBlinding", input_blinding.clone()),
			("inputNullifier", input_nullifier.clone()),
			("inputRoot", input_root.clone()),
			("inputPathElements", input_path_elements.clone()),
			("inputPathIndices", input_path_indices.clone()),
			("outputChainID", output_chain_id.clone()),
			("outputAmount", output_amount.clone()),
			("outputPrivateKey", output_private_key.clone()),
			("outputBlinding", output_blinding.clone()),
			("outputCommitment", output_commitment.clone()),
			("unspentTimestamp", unspent_timestamp.clone()),
			("unspentRoots", unspent_roots.clone()),
			("unspentPathIndices", unspent_path_indices.clone()),
			("unspentPathElements", unspent_path_elements.clone()),
			("spentTimestamp", spent_timestamp.clone()),
			("spentRoots", spent_roots.clone()),
			("spentPathIndices", spent_path_indices.clone()),
			("spentPathElements", spent_path_elements.clone()),
		];
		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let mut inputs_for_verification = &full_assignment[1..num_inputs];

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);
	});
}
