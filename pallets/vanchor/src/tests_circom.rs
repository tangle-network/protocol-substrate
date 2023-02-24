use crate::{
	mock::*,
	test_utils::{deconstruct_public_inputs_el, setup_utxos, ANCHOR_CT, DEFAULT_LEAF, NUM_UTXOS},
	tests::*,
	zerokit_utils::*,
	Instance2,
};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_circom::{read_zkey, CircomConfig, CircomReduction, WitnessCalculator};
use ark_ff::{BigInteger, BigInteger256, PrimeField, ToBytes};
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
	linkable_tree::LinkableTreeInspector,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::compute_chain_id_type,
	verifying::CircomError,
	AccountId,
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
	// 1. Setup The Hasher Pallet.
	println!("Setting up the hasher pallet");
	assert_ok!(Hasher2::force_set_parameters(
		RuntimeOrigin::root(),
		params3.to_bytes().try_into().unwrap()
	));
	// 2. Initialize MerkleTree pallet.
	println!("Initializing the merkle tree pallet");
	<MerkleTree2 as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	// Load the WASM and R1CS for witness and proof generation
	// Get path to solidity fixtures
	println!("Setting up the verifier pallet");
	// let wasm_2_2_path = fs::canonicalize(
	// 	"../../solidity-fixtures/solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.wasm",
	// );
	// let r1cs_2_2_path = fs::canonicalize(
	// 	"../../solidity-fixtures/solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.r1cs",
	// );
	// println!("Setting up CircomConfig");
	// println!("wasm_2_2_path: {:?}", wasm_2_2_path);
	// println!("r1cs_2_2_path: {:?}", r1cs_2_2_path);
	// let cfg_2_2 =
	// 	CircomConfig::<Bn254>::new(wasm_2_2_path.unwrap(), r1cs_2_2_path.unwrap()).unwrap();

	println!("Setting up ZKey");
	let path_2_2 = "../../solidity-fixtures/solidity-fixtures/vanchor_2/2/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	let wasm_2_2_path =
		"../../solidity-fixtures/solidity-fixtures//vanchor_2/2/poseidon_vanchor_2_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

	let mut vk_2_2_bytes = Vec::new();
	params_2_2.0.vk.write(&mut vk_2_2_bytes).unwrap();
	// println!("vk_2_2_bytes: {:?}", vk_2_2_bytes.len());
	// println!("vk: {:?}", params_2_2.0.vk);

	assert_ok!(VAnchorVerifier2::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 2),
		vk_2_2_bytes.clone().try_into().unwrap()
	));

	let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	let relayer = account::<AccountId>("", RELAYER_ACCOUNT_ID, SEED);
	let big_transactor = account::<AccountId>("", BIG_TRANSACTOR_ACCOUNT_ID, SEED);
	let bigger_transactor = account::<AccountId>("", BIGGER_TRANSACTOR_ACCOUNT_ID, SEED);

	// Set balances
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), transactor, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), relayer, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		big_transactor,
		BIG_DEFAULT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		bigger_transactor,
		BIGGER_DEFAULT_BALANCE,
		0
	));

	// set configurable storage
	assert_ok!(VAnchor2::set_max_deposit_amount(RuntimeOrigin::root(), 10, 1));
	assert_ok!(VAnchor2::set_min_withdraw_amount(RuntimeOrigin::root(), 3, 2));

	// finally return the provingkey bytes
	(params_2_2, wc_2_2)
}

fn insert_utxos_to_merkle_tree(
	utxos: &[Utxo<Bn254Fr>; 2],
	neighbor_roots: [Element; ANCHOR_CT - 1],
	custom_root: Element,
) -> (
	[u64; 2],
	[Vec<u8>; 2],
	SparseMerkleTree<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>,
	Vec<Path<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>>,
) {
	let curve = Curve::Bn254;
	let leaf0 = utxos[0].commitment.into_repr().to_bytes_be();
	let leaf1 = utxos[1].commitment.into_repr().to_bytes_be();

	let leaves: Vec<Vec<u8>> = vec![leaf0, leaf1];
	let leaves_f: Vec<Bn254Fr> =
		leaves.iter().map(|x| Bn254Fr::from_be_bytes_mod_order(x)).collect();

	let in_indices = [0, 1];

	let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
	let poseidon3 = Poseidon::new(params3);
	let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
		&poseidon3,
		&leaves_f,
		0,
		&DEFAULT_LEAF,
	)
	.unwrap();

	let in_paths: Vec<_> = in_indices.iter().map(|i| tree.generate_membership_proof(*i)).collect();

	let roots_f: [Bn254Fr; ANCHOR_CT] = vec![if custom_root != Element::from_bytes(&[0u8; 32]) {
		Bn254Fr::from_be_bytes_mod_order(custom_root.to_bytes())
	} else {
		tree.root()
	}]
	.iter()
	.chain(
		neighbor_roots
			.iter()
			.map(|r| Bn254Fr::from_be_bytes_mod_order(r.to_bytes()))
			.collect::<Vec<Bn254Fr>>()
			.iter(),
	)
	.cloned()
	.collect::<Vec<Bn254Fr>>()
	.try_into()
	.unwrap();
	let in_root_set = roots_f.map(|x| x.into_repr().to_bytes_be());

	(in_indices, in_root_set, tree, in_paths)
}

pub fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = EDGE_CT as u32;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor2::create(RuntimeOrigin::root(), max_edges, depth, asset_id));
	MerkleTree2::next_tree_id() - 1
}

#[test]
fn circom_should_complete_2x2_transaction_with_withdraw() {
	new_test_ext().execute_with(|| {
		let params4 = setup_params::<Bn254Fr>(Curve::Bn254, 5, 4);
		let nullifier_hasher = Poseidon::<Bn254Fr> { params: params4 };
		let (params_2_2, wc_2_2) = setup_environment_with_circom();
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = 10_i128;
		let public_amount = 10_i128;
		let fee: Balance = 0;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);
		println!("ext_data: {:?}", ext_data);
		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree2::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree2 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance2>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();
		println!("neighbor_roots: {:?}", neighbor_roots);

		let input_nullifiers = in_utxos
			.clone()
			.map(|utxo| utxo.calculate_nullifier(&nullifier_hasher).unwrap());

		let (in_indices, _in_root_set, _tree, in_paths) =
			insert_utxos_to_merkle_tree(&in_utxos, neighbor_roots, custom_root);

		// Make Inputs
		let public_amount = if public_amount > 0 {
			vec![BigInt::from_bytes_be(Sign::Plus, &public_amount.to_be_bytes())]
		} else {
			vec![BigInt::from_bytes_be(Sign::Minus, &(-public_amount).to_be_bytes())]
		};
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);
		let ext_data_hash = vec![BigInt::from_bytes_be(Sign::Plus, &keccak_256(&ext_data.encode_abi()))];

		// let mut ext_data_hash = public_amount.clone();
		let mut input_nullifier = Vec::new();
		let mut output_commitment = Vec::new();
		for i in 0..NUM_UTXOS {
			input_nullifier.push(BigInt::from_bytes_be(
				Sign::Plus,
				&input_nullifiers[i].into_repr().to_bytes_be(),
			));
			output_commitment.push(BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].commitment.into_repr().to_bytes_be(),
			));
		}

		let mut chain_id = vec![BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes())];

		let mut roots = Vec::new();

		roots.push(BigInt::from_bytes_be(Sign::Plus, &custom_root.0));
		for i in 0..ANCHOR_CT - 1 {
			roots.push(BigInt::from_bytes_be(Sign::Plus, &neighbor_roots[i].0));
		}

		let mut in_amount = Vec::new();
		let mut in_private_key = Vec::new();
		let mut in_blinding = Vec::new();
		let mut in_path_indices = Vec::new();
		let mut in_path_elements = Vec::new();
		let mut out_chain_id = Vec::new();
		let mut out_amount = Vec::new();
		let mut out_pub_key = Vec::new();
		let mut out_blinding = Vec::new();

		for i in 0..NUM_UTXOS {
			in_amount.push(BigInt::from_bytes_be(
				Sign::Plus,
				&in_utxos[i].amount.into_repr().to_bytes_be(),
			));
			in_private_key.push(BigInt::from_bytes_be(
				Sign::Plus,
				&in_utxos[i].keypair.secret_key.unwrap().into_repr().to_bytes_be(),
			));
			in_blinding.push(BigInt::from_bytes_be(
				Sign::Plus,
				&in_utxos[i].blinding.into_repr().to_bytes_be(),
			));
			in_path_indices.push(BigInt::from(in_indices[i]));
			for j in 0..TREE_DEPTH {
				let neighbor_elt: Bn254Fr =
					if in_indices[i] == 0 { in_paths[i].path[j].1 } else { in_paths[i].path[j].0 };
				in_path_elements.push(BigInt::from_bytes_be(
					Sign::Plus,
					&neighbor_elt.into_repr().to_bytes_be(),
				));
			}

			out_chain_id.push(BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].chain_id.into_repr().to_bytes_be(),
			));

			out_amount.push(BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].amount.into_repr().to_bytes_be(),
			));

			out_pub_key.push(BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].keypair.public_key.into_repr().to_bytes_be(),
			));

			out_blinding.push(BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].blinding.into_repr().to_bytes_be(),
			));
		}

		let inputs_for_proof = [
			("publicAmount", public_amount.clone()),
			("extDataHash", ext_data_hash.clone()),
			("inputNullifier", input_nullifier.clone()),
			("inAmount", in_amount.clone()),
			("inPrivateKey", in_private_key.clone()),
			("inBlinding", in_blinding.clone()),
			("inPathIndices", in_path_indices.clone()),
			("inPathElements", in_path_elements.clone()),
			("outputCommitment", output_commitment.clone()),
			("outChainID", out_chain_id.clone()),
			("outAmount", out_amount.clone()),
			("outPubkey", out_pub_key.clone()),
			("outBlinding", out_blinding.clone()),
			("chainID", chain_id.clone()),
			("roots", roots.clone()),
		];

		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let mut inputs_for_verification = &full_assignment[1..num_inputs];

		println!(
			"v {:?} {:?}",
			inputs_for_verification.len(),
			inputs_for_verification
				.into_iter()
				.map(|x| to_bigint(&x))
				.collect::<Vec<BigInt>>()
		);

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&inputs_for_verification.to_vec());
		println!("full assignment {:?}", full_assignment[1..num_inputs].to_vec());
		println!("inputs for proof {:?}", inputs_for_proof);
		let mut proof_bytes = Vec::new();
		proof.write(&mut proof_bytes).unwrap();
		let proof_data =
			ProofData::new(proof_bytes, public_amount, root_set, nullifiers, commitments, ext_data_hash);
				println!("Proof data: {proof_data:?}");

		let _relayer_balance_before = Balances::free_balance(relayer.clone());
		let _recipient_balance_before = Balances::free_balance(recipient.clone());
		let _transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_ok!(VAnchor2::transact(
			RuntimeOrigin::signed(transactor.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		// // Constructing external data
		// let output1 = commitments[0];
		// let output2 = commitments[1];
		// let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
		// 	recipient.clone(),
		// 	relayer.clone(),
		// 	ext_amount,
		// 	fee,
		// 	0,
		// 	0,
		// 	output1.to_vec(),
		// 	output2.to_vec(),
		// );

		// let relayer_balance_before = Balances::free_balance(relayer.clone());
		// let recipient_balance_before = Balances::free_balance(recipient.clone());
		// let transactor_balance_before = Balances::free_balance(transactor.clone());
		// assert_ok!(VAnchor2::transact(
		// 	RuntimeOrigin::signed(transactor.clone()),
		// 	tree_id,
		// 	proof_data,
		// 	ext_data
		// ));

		// // Recipient balance should be ext amount since the fee was zero
		// let recipient_balance_after = Balances::free_balance(recipient);
		// assert_eq!(recipient_balance_after, recipient_balance_before);

		// // Relayer balance should be zero since the fee was zero
		// let relayer_balance_after = Balances::free_balance(relayer);
		// assert_eq!(relayer_balance_after, relayer_balance_before);

		// // Transactor balance should be zero, since they deposited all the
		// // money to the mixer
		// let transactor_balance_after = Balances::free_balance(transactor);
		// assert_eq!(transactor_balance_after, transactor_balance_before -
		// ext_amount.unsigned_abs());
	});
}
