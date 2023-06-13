use core::str::FromStr;

use crate::{
	mock::*,
	test_utils::{
		deconstruct_public_inputs_el, setup_environment_with_circom, setup_utxos, ANCHOR_CT,
		DEFAULT_LEAF, NUM_UTXOS, TREE_DEPTH,
	},
	tests::*,
	Instance2,
};
// use ark_bn254::{Bn254, Fr};
// use ark_circom::{read_zkey, WitnessCalculator};
use ark_ff::{BigInteger, PrimeField};
// use ark_groth16::ProvingKey;
// use ark_relations::r1cs::ConstraintMatrices;
use ark_serialize::CanonicalSerialize;
use arkworks_native_gadgets::{
	merkle_tree::{Path, SparseMerkleTree},
	poseidon::Poseidon,
};
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path},
	utxo::Utxo,
	Curve,
};
use circom_proving::{generate_proof, verify_proof};
use codec::{Decode, Encode};
use frame_support::assert_ok;
use num_bigint::{BigInt, Sign};
use pallet_linkable_tree::LinkableTreeConfigration;
use sp_core::hashing::keccak_256;

use webb_primitives::{
	linkable_tree::LinkableTreeInspector,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::compute_chain_id_type,
	AccountId,
};

type Bn254Fr = ark_bn254::Fr;

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
		println!("ext_data: {ext_data:?}");

		let custom_root = MerkleTree2::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree2 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance2>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();
		println!("neighbor_roots: {neighbor_roots:?}");

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

		let ext_data_hash =
			vec![BigInt::from_bytes_be(Sign::Plus, keccak_256(&ext_data.encode_abi()).as_slice())];

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

		let chain_id = vec![BigInt::from_bytes_be(Sign::Plus, &chain_id.to_be_bytes())];

		let mut roots = Vec::new();

		roots.push(BigInt::from_bytes_be(Sign::Plus, &custom_root.0));
		#[allow(clippy::needless_range_loop)]
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
			("publicAmount", public_amount),
			("extDataHash", ext_data_hash),
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
			("chainID", chain_id),
			("roots", roots.clone()),
		];

		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let inputs_for_verification = &full_assignment[1..num_inputs];

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);

		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&inputs_for_verification.to_vec());
		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();
		let proof_data = ProofData::new(
			proof_bytes,
			public_amount,
			root_set,
			nullifiers,
			commitments,
			ext_data_hash,
		);
		println!("Proof data: {proof_data:?}");

		let _relayer_balance_before = Balances::free_balance(relayer);
		let _recipient_balance_before = Balances::free_balance(recipient);
		let _transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_ok!(VAnchor2::transact(
			RuntimeOrigin::signed(transactor),
			tree_id,
			proof_data,
			ext_data
		));
	});
}

#[test]
fn javascript_ext_data_to_rust() {
	let recipient_bytes =
		hex::decode("306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20").unwrap();
	let recipient = sp_runtime::AccountId32::decode(&mut &recipient_bytes[..]).unwrap();
	let relayer_bytes =
		hex::decode("306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20").unwrap();
	let relayer = sp_runtime::AccountId32::decode(&mut &relayer_bytes[..]).unwrap();
	let ext_amount_bytes = hex::decode("000000000000003635c9adc5dea00000").unwrap();
	let ext_amount = Amount::decode(&mut &ext_amount_bytes[..]).unwrap();
	let fee_bytes = hex::decode("00000000000000000000000000000000").unwrap();
	let fee = Balance::decode(&mut &fee_bytes[..]).unwrap();
	let refund_bytes = hex::decode("00000000000000000000000000000000").unwrap();
	let refund = Balance::decode(&mut &refund_bytes[..]).unwrap();
	let token_bytes = hex::decode("00000000").unwrap();
	let token = AssetId::decode(&mut &token_bytes[..]).unwrap();
	let output1_bytes = hex::decode("1e4a6679e64b8e495d0c620bb2e4406bd381720c8486625fc2ff8d6cbe8c8b76064507585462a045b7c118248e7f271cde43aa7b0d94b70e8364033b86f80ec3a720105caf6d263c30b22872524fb560b475d94ba98eb1c6b5d633bfeb5cae22c7ede2f24e962f2bb2dbcb3fcddb9edbcdb5b71a02480d700d7cfec9a7512bc31d0349d5a9972a13d083499525e6bf2ac7c19df17b2f5871c9271775f9e398da86577d1fa387de18").unwrap();
	let output2_bytes = hex::decode("dccdba14215029d938d431c2838c1714ab7ac3750973d0aabee6fae182b40245e3e57b04123ecc4498775c8e47d0f96601a28f20a63301066827edb055088fe0475b41f6edfd678171ee8071e527a7593a308bc4650bf61f8d2a367119460345332466a9544d447064ecff5811492c0a5124b049495169dc16ec8c5cf8e2f049ab67f00466bc5c39d675498f3be66a712010c1c737f37a8e2e53ef1c3b81eb1006533fe4223fa95e").unwrap();
	let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
		recipient,
		relayer,
		ext_amount,
		fee,
		refund,
		token,
		output1_bytes.to_vec(),
		output2_bytes.to_vec(),
	);
	println!("ext_data: {ext_data:?}");
	println!("ext_data.encode_abi(): {}", hex::encode(ext_data.encode_abi()));

	let ext_data_hash = keccak_256(&ext_data.encode_abi());
	let ext_data_hash_hex = hex::encode(ext_data_hash);
	let expected_ext_data_hash_hex =
		"247f12db5bf0a9f24dffc39e6ce4259a746bd0d79916461614219ea6a90ee674";
	println!("ext_data_hash_hex: {ext_data_hash_hex}");
	println!("expected_ext_data_hash_hex: {expected_ext_data_hash_hex}");
	assert_eq!(ext_data_hash_hex, expected_ext_data_hash_hex);
	let ext_data_hash_bigint = BigInt::from_bytes_be(Sign::Plus, ext_data_hash.as_slice());
	println!("ext_data_hash_bigint: {ext_data_hash_bigint}");
	let expected_ext_data_hash_bigint = BigInt::from_str("16507782271569429036213645335826041130156501527126637127174074489240854390388").unwrap();
	assert_eq!(ext_data_hash_bigint, expected_ext_data_hash_bigint);
}
