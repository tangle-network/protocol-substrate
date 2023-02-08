use crate::{
	mock::*,
	test_utils::{deconstruct_public_inputs_el, setup_utxos, ANCHOR_CT, DEFAULT_LEAF, NUM_UTXOS},
	tests::*,
	Instance2,
	zerokit_utils::*,
};
use cfg_if::cfg_if;
use ark_relations::r1cs::ConstraintMatrices;
use ark_relations::r1cs::SynthesisError;
use ark_bn254::{Fr, Bn254};
use once_cell::sync::OnceCell;
use std::sync::Mutex;
use wasmer::{Module, Store};
use ark_circom::{read_zkey, WitnessCalculator, CircomReduction, CircomConfig};
use ark_ff::{BigInteger, PrimeField, ToBytes};
use ark_groth16::{ Proof as ArkProof, create_proof_with_reduction_and_matrices, VerifyingKey,
	create_random_proof as prove, generate_random_parameters, prepare_verifying_key,
	ProvingKey, verify_proof as ark_verify_proof,
};
use std::io::{Cursor, Error, ErrorKind};
use std::result::Result;
use thiserror::Error;
use arkworks_native_gadgets::{
	merkle_tree::{Path, SparseMerkleTree},
	poseidon::Poseidon,
};
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path},
	utxo::Utxo,
	Curve,
};
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::OnInitialize};
use num_bigint::{BigInt, Sign};
use pallet_linkable_tree::LinkableTreeConfigration;
use ark_std::{rand::thread_rng, UniformRand};
use sp_core::hashing::keccak_256;
use std::{
	convert::TryInto,
	fs::{self, File},
};
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

pub fn generate_proof(
    #[cfg(not(target_arch = "wasm32"))] witness_calculator: &Mutex<WitnessCalculator>,
    #[cfg(target_arch = "wasm32")] witness_calculator: &mut WitnessCalculator,
    proving_key: &(ProvingKey<Bn254>, ConstraintMatrices<Fr>),
    vanchor_witness: [(&str, Vec<BigInt>); 15],
) -> Result<ArkProof<Bn254>, ProofError> {
    let inputs = vanchor_witness
        .into_iter()
        .map(|(name, values)| (name.to_string(), values.clone()));


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

    Ok(proof)
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


fn setup_environment_with_circom() -> ((ProvingKey<Bn254>, ConstraintMatrices<Fr>), &'static Mutex<WitnessCalculator>) {
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

	let wasm_2_2_path = "../../solidity-fixtures/solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

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

		let mut ext_data_hash = vec![BigInt::from_bytes_be(Sign::Plus, &ext_data_hash)];
		let mut input_nullifier = Vec::new();
		let mut output_commitment = Vec::new();
		for i in 0..NUM_UTXOS {
			input_nullifier.push(
				BigInt::from_bytes_be(Sign::Plus, &input_nullifiers[i].into_repr().to_bytes_be()),
			);
			output_commitment.push(
				BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].commitment.into_repr().to_bytes_be()),
			);
		}

		let mut chain_id = vec![BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes())];

		let mut roots = Vec::new();

		roots.push(BigInt::from_bytes_be(Sign::Plus, &custom_root.0));
		for i in 0..ANCHOR_CT - 1 {
			roots.push(BigInt::from_bytes_be(Sign::Plus, &neighbor_roots[i].0));
		}

		 let mut in_amount= Vec::new();
		 let mut in_private_key= Vec::new();
		 let mut in_blinding= Vec::new();
		 let mut in_path_indices= Vec::new();
		 let mut in_path_elements = Vec::new();
		 let mut out_chain_id= Vec::new();
		 let mut out_amount= Vec::new();
		 let mut out_pub_key = Vec::new();
		 let mut out_blinding = Vec::new();

		for i in 0..NUM_UTXOS {
			in_amount.push(
				BigInt::from_bytes_be(Sign::Plus, &in_utxos[i].amount.into_repr().to_bytes_be()),
			);
			in_private_key.push(
				BigInt::from_bytes_be(
					Sign::Plus,
					&in_utxos[i].keypair.secret_key.unwrap().into_repr().to_bytes_be(),
				),
			);
			in_blinding.push(
				BigInt::from_bytes_be(Sign::Plus, &in_utxos[i].blinding.into_repr().to_bytes_be()),
			);
			in_path_indices.push(BigInt::from(in_indices[i]));
			for j in 0..TREE_DEPTH {
				let neighbor_elt: Bn254Fr =
					if in_indices[i] == 0 { in_paths[i].path[j].1 } else { in_paths[i].path[j].0 };
				in_path_elements.push(
					BigInt::from_bytes_be(Sign::Plus, &neighbor_elt.into_repr().to_bytes_be()),
				);
			}
		
			out_chain_id.push(
				BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].chain_id.into_repr().to_bytes_be()),
			);

			out_amount.push(
				BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].amount.into_repr().to_bytes_be()),
			);

			out_pub_key.push(
				BigInt::from_bytes_be(
					Sign::Plus,
					&out_utxos[i].keypair.public_key.into_repr().to_bytes_be(),
				),
			);

			out_blinding.push(
				BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].blinding.into_repr().to_bytes_be()),
			);
		}

		let inputs_for_proof = [
			("public_amount", public_amount.clone()),
			("ext_data_hash", ext_data_hash.clone()),
			("input_nullifier", input_nullifier.clone()),
			("output_commitment", output_commitment.clone()),
			("chain_id", chain_id.clone()),
			("roots", roots.clone()),
			("in_amount", in_amount.clone()),
			("in_private_key", in_private_key.clone()),
			("in_blinding", in_blinding.clone()),
			("in_path_indices", in_path_indices.clone()),
			("in_path_elements", in_path_elements.clone()),
			("out_chain_id", out_chain_id.clone()),
			("out_amount", out_amount.clone()),
			("out_pub_key", out_pub_key.clone()),
			("out_blinding", out_blinding.clone()),
		];

		let proof = generate_proof(wc_2_2, &params_2_2, inputs_for_proof);
		
		let mut inputs_for_verification = Vec::new();

		for x in public_amount.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in ext_data_hash.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in input_nullifier.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in output_commitment.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in chain_id.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in roots.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in in_amount.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in in_private_key.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in in_blinding.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in in_path_indices.iter() {
			inputs_for_verification.push(from_bigint(x));
		}
		for x in in_path_elements.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in out_chain_id.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in out_amount.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in out_pub_key.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		for x in out_blinding.iter() {
			inputs_for_verification.push(from_bigint(x));
		}

		verify_proof(&params_2_2.0.vk, &proof.unwrap(), inputs_for_verification);
		

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
		// assert_eq!(transactor_balance_after, transactor_balance_before - ext_amount.unsigned_abs());
	});
}
