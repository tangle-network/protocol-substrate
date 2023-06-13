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
	// {
	//   recipient: '306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20',
	//   relayer: '306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20',
	//   extAmount: '000000000000003635c9adc5dea00000',
	//   fee: '00000000000000000000000000000000',
	//   refund: '00000000000000000000000000000000',
	//   token: '00000000',
	//   encryptedOutput1:
	// '77c4ee470e581dab9d70393103bfdccac0e2cb9e5ceec7ce96a1563a9f77ebdfa5bb0194c37fc985d4d80312e151d8a3a2b6207a4aee9b370d6b58e29ee5c8045dbd0d60790d18e0d98b89fbdf25fbd88a0e9747e1f27ddaae0903fbadfb7dff8792a3425000693a27a50fd4a56db4be8d315ead7af591b2fb594bb7621bd337455f8c11e89f4c048f58993bd9a93de774977e9aa112b0cfd18981161fbeee9a91ae1f550a601c97'
	// ,   encryptedOutput2:
	// '21b190a5515998333e4e4f79f8c9063ade8fc794a8967a2a1e08416d3afd97c4a5947462f0ada64013c44a13ca0b6d4898bbdf9ce9fa8d5016771afd3931e99a8d5c3d25a7c0015bd8dc44cfa5f8da1b1357eca7ee12efe74209cfdfde9ceb835917558c418f06db2352a2266cbfed0099b48f63ca53ce19a59eefa03edfdb5ca1b747350200e1cd29b6dd2e133c0d119ffb114c6245e81bbc8a890c4d797cbcdc274b16a017aa68'
	// }
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
	let output1_bytes = hex::decode("7a7cde632298bdb5f135bc50c8363ddac154658222efd8df0b6f631a4a2ca80dfb8e307622bfb1f10eda7346d4c4d5ee0501ae4f84c1a34932ead2c54468da9e124d36f9092d0613a903be16e1187c6f51ce8b797c20b5afc01df31ca2c2ccd614195d38dcd22af22f5d36943f23a4f75672654f49a04c10cfdd05afa023b3d14025460cb362f62acc3ec0a2d293df3adddecadfb1c31a8c37118b0f5e37e90ce2f3e33ab6d8052d").unwrap();
	let output2_bytes = hex::decode("88be3138c3af2dbdadd2888eeb0b3a32a1bba77ca29afb9a9f38faffa5e924d97a8b4097b18e5c98652d1b96ed824c8204d50115ad15f028b0bef2067797d868cd6ef7995ff04da98994181b157c93c6f205ffde4cbefc9eefdb2434129fe639c54d0e92db5471621a052aa3e1e8bb86b625b2954c5794de3f5bbb2cac6b1e011b816306a8ae49f1c274bcad8b9261fd28b6f250a0b304ea291de7d352efef900e51e2ec819b5d7e").unwrap();
	let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
		recipient,
		relayer,
		ext_amount,
		fee,
		refund,
		token,
		// Mock encryption value, not meant to be used in production
		output1_bytes.to_vec(),
		// Mock encryption value, not meant to be used in production
		output2_bytes.to_vec(),
	);
	println!("ext_data: {ext_data:?}");
	println!("ext_data.encode_abi(): {}", hex::encode(ext_data.encode_abi()));

	let ext_data_hash = keccak_256(&ext_data.encode_abi());
	let ext_data_hash_hex = hex::encode(ext_data_hash);
	let expected_ext_data_hash_hex =
		"392a72b9fbb5cf94eb5af9e27519bffc42420f9f83af73ea2729ff1565516796";
	println!("ext_data_hash_hex: {ext_data_hash_hex}");
	println!("expected_ext_data_hash_hex: {expected_ext_data_hash_hex}");
	assert_eq!(ext_data_hash_hex, expected_ext_data_hash_hex);
	let ext_data_hash = vec![BigInt::from_bytes_be(Sign::Plus, ext_data_hash.as_slice())];
}
