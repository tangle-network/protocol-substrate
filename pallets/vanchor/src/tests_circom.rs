use crate::{
	mock::*,
	test_utils::{deconstruct_public_inputs_el, setup_utxos, ANCHOR_CT, DEFAULT_LEAF, NUM_UTXOS},
	tests::*,
	Instance2,
};
use ark_bn254::Bn254;
use ark_circom::{read_zkey, CircomBuilder, CircomConfig};
use ark_ff::{BigInteger, PrimeField, ToBytes};
use ark_groth16::{
	create_random_proof as prove, generate_random_parameters, prepare_verifying_key, verify_proof,
	ProvingKey,
};
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
use rand::thread_rng;
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

fn setup_environment_with_circom() -> (Vec<u8>, ProvingKey<Bn254>, CircomConfig<Bn254>) {
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
	let wasm_2_2_path = fs::canonicalize(
		"../../solidity-fixtures/solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.wasm",
	);
	let r1cs_2_2_path = fs::canonicalize(
		"../../solidity-fixtures/solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.r1cs",
	);
	println!("Setting up CircomConfig");
	println!("wasm_2_2_path: {:?}", wasm_2_2_path);
	println!("r1cs_2_2_path: {:?}", r1cs_2_2_path);
	let cfg_2_2 =
		CircomConfig::<Bn254>::new(wasm_2_2_path.unwrap(), r1cs_2_2_path.unwrap()).unwrap();

	println!("Setting up ZKey");
	let path_2_2 = "../../solidity-fixtures/solidity-fixtures/vanchor_2/2/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let (params_2_2, _matrices) = read_zkey(&mut file_2_2).unwrap();

	let mut vk_2_2_bytes = Vec::new();
	params_2_2.vk.write(&mut vk_2_2_bytes).unwrap();
	println!("vk_2_2_bytes: {:?}", vk_2_2_bytes.len());

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
	(vk_2_2_bytes, params_2_2, cfg_2_2)
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

pub fn setup_circom_zk_circuit(
	config: CircomConfig<Bn254>,
	public_amount: i128,
	chain_id: u64,
	ext_data_hash: Vec<u8>,
	in_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	out_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	_proving_key: ProvingKey<Bn254>,
	neighbor_roots: [Element; ANCHOR_CT - 1],
	custom_root: Element,
) -> Result<(Vec<u8>, Vec<Bn254Fr>), CircomError> {
	use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};

	let (in_indices, _in_root_set, _tree, in_paths) =
		insert_utxos_to_merkle_tree(&in_utxos, neighbor_roots, custom_root);

	let params4 = setup_params::<Bn254Fr>(Curve::Bn254, 5, 4);
	let nullifier_hasher = Poseidon::<Bn254Fr> { params: params4 };
	let input_nullifiers = in_utxos
		.clone()
		.map(|utxo| utxo.calculate_nullifier(&nullifier_hasher).unwrap());

	let mut builder = CircomBuilder::new(config);
	// Public inputs
	// publicAmount, extDataHash, inputNullifier, outputCommitment, chainID, roots
	builder.push_input(
		"publicAmount",
		if public_amount > 0 {
			BigInt::from_bytes_be(Sign::Plus, &public_amount.to_be_bytes())
		} else {
			BigInt::from_bytes_be(Sign::Minus, &(-public_amount).to_be_bytes())
		}
	);
	builder.push_input("extDataHash", BigInt::from_bytes_be(Sign::Plus, &ext_data_hash));
	for i in 0..NUM_UTXOS {
		builder.push_input(
			"inputNullifier",
			BigInt::from_bytes_be(Sign::Plus, &input_nullifiers[i].into_repr().to_bytes_be()),
		);
		builder.push_input(
			"outputCommitment",
			BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].commitment.into_repr().to_bytes_be()),
		);
	}
	builder.push_input("chainID", BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes()));
	builder.push_input("roots", BigInt::from_bytes_be(Sign::Plus, &custom_root.0));
	for i in 0..ANCHOR_CT - 1 {
		builder.push_input("roots", BigInt::from_bytes_be(Sign::Plus, &neighbor_roots[i].0));
	}
	// Private inputs
	// inAmount, inPrivateKey, inBlinding, inPathIndices, inPathElements
	// outChainID, outAmount, outPubkey, outBlinding
	for i in 0..NUM_UTXOS {
		builder.push_input(
			"inAmount",
			BigInt::from_bytes_be(Sign::Plus, &in_utxos[i].amount.into_repr().to_bytes_be()),
		);
		builder.push_input(
			"inPrivateKey",
			BigInt::from_bytes_be(
				Sign::Plus,
				&in_utxos[i].keypair.secret_key.unwrap().into_repr().to_bytes_be(),
			),
		);
		builder.push_input(
			"inBlinding",
			BigInt::from_bytes_be(Sign::Plus, &in_utxos[i].blinding.into_repr().to_bytes_be()),
		);
		builder.push_input("inPathIndices", BigInt::from(in_indices[i]));
		for j in 0..TREE_DEPTH {
			let neighbor_elt: Bn254Fr =
				if in_indices[i] == 0 { in_paths[i].path[j].1 } else { in_paths[i].path[j].0 };
			builder.push_input(
				"inPathElements",
				BigInt::from_bytes_be(Sign::Plus, &neighbor_elt.into_repr().to_bytes_be()),
			);
		}

		builder.push_input(
			"outChainID",
			BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].chain_id.into_repr().to_bytes_be()),
		);
		builder.push_input(
			"outAmount",
			BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].amount.into_repr().to_bytes_be()),
		);
		builder.push_input(
			"outPubkey",
			BigInt::from_bytes_be(
				Sign::Plus,
				&out_utxos[i].keypair.public_key.into_repr().to_bytes_be(),
			),
		);
		builder.push_input(
			"outBlinding",
			BigInt::from_bytes_be(Sign::Plus, &out_utxos[i].blinding.into_repr().to_bytes_be()),
		);
	}

	let mut rng = thread_rng();
	// Run a trusted setup
	let circom = builder.setup();
	let params = generate_random_parameters::<Bn254, _, _>(circom.clone(), &mut rng)
		.map_err(|_e| CircomError::ParameterGenerationFailure)?;
	// let circom = builder.build().map_err(|_e| CircomError::InvalidBuilderConfig)?;
	let cs = ConstraintSystem::<Bn254Fr>::new_ref();
	circom.clone().generate_constraints(cs.clone()).unwrap();
	let is_satisfied = cs.is_satisfied().unwrap();
	println!("is satisfied: {}", is_satisfied);
	if !is_satisfied {
		println!("Unsatisfied constraint: {:?}", cs.which_is_unsatisfied().unwrap());
	}

	let inputs = circom.get_public_inputs().unwrap();
	println!("inputs: {:?}", inputs.len());
	// Generate the proof
	let mut proof_bytes = vec![];
	let proof = prove(circom, &params, &mut rng).map_err(|_e| CircomError::ProvingFailure)?;
	// let proof = prove(circom, &_proving_key, &mut rng).map_err(|_e|
	// CircomError::ProvingFailure)?;
	proof.write(&mut proof_bytes).unwrap();
	let pvk = prepare_verifying_key(&params.vk);
	// let pvk = _proving_key.vk.into();
	let verified =
		verify_proof(&pvk, &proof, &inputs).map_err(|_e| CircomError::VerifyingFailure)?;

	assert!(verified, "Proof is not verified");

	Ok((proof_bytes, inputs))
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
		let (_, params_2_2, cfg_2_2) = setup_environment_with_circom();
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
		let (proof, public_inputs) = setup_circom_zk_circuit(
			cfg_2_2,
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			params_2_2,
			neighbor_roots,
			custom_root,
		)
		.unwrap();

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_ok!(VAnchor2::transact(
			RuntimeOrigin::signed(transactor.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		// Recipient balance should be ext amount since the fee was zero
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should be zero, since they deposited all the
		// money to the mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before - ext_amount.unsigned_abs());
	});
}
