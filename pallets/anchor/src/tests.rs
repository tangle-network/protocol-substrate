use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalSerialize;
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	setup::{
		bridge::{
			prove_groth16_circuit_circomx5, setup_arbitrary_data, setup_circuit_circomx5,
			setup_groth16_circuit_circomx5, setup_leaf_circomx5, setup_set, Circuit_Circomx5,
		},
		common::{setup_circom_params_x5_3, setup_circom_params_x5_5, setup_tree_and_create_path_tree_circomx5, Curve},
	},
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
const TEST_MAX_EDGES: u32 = 30;
use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	merkle_tree::TreeInspector,
	ElementTrait,
};
use frame_support::{assert_ok, traits::OnInitialize};

use crate::mock::*;

const LEN: usize = 30;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

#[test]
fn should_create_new_anchor() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		let deposit_size = 10_000;
		let max_edges = 2;
		let depth = LEN as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), deposit_size, max_edges, depth, asset_id));
	});
}

#[test]
fn should_be_able_to_deposit() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);

		let deposit_size = 10_000;
		let max_edges = 2;
		let depth = LEN as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), deposit_size, max_edges, depth, asset_id));

		let tree_id = MerkleTree::next_tree_id() - 1;
		let account_id = 1;
		let _ = Balances::set_balance(Origin::root(), account_id, 100_000_000, 0);

		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id);
		println!("Balance before: {}", balance_before);
		// and we do the deposit
		assert_ok!(Anchor::deposit(Origin::signed(account_id), tree_id, leaf));
		// now we check the balance after the deposit.
		let balance_after = Balances::free_balance(account_id);
		// the balance should be less now with `deposit_size`
		assert_eq!(balance_after, balance_before - deposit_size);
		// now we need also to check if the state got updated.
		let tree = MerkleTree::trees(tree_id);
		assert_eq!(tree.leaf_count, 1);
	});
}

#[test]
fn should_be_able_to_change_the_maintainer() {
	new_test_ext().execute_with(|| {
		let deposit_size = 10_000;
		let max_edges = 2;
		let depth = LEN as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), deposit_size, max_edges, depth, asset_id));

		let default_maintainer_account_id = 0;
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);
		let new_maintainer_account_id = 1;
		assert_ok!(Anchor::force_set_maintainer(Origin::root(), new_maintainer_account_id));
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}

#[test]
fn should_be_able_to_add_neighbors_and_check_history() {
	use rand::prelude::*;

	new_test_ext().execute_with(|| {
		let deposit_size = 10_000;
		let max_edges = 2;
		let depth = LEN as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), deposit_size, max_edges, depth, asset_id));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let src_chain_id = 1;
		let root = Element::from_bytes(&[1u8; 32]);
		let height = 1;
		let neighbor_index_before: u32 = Anchor::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_ok!(<Anchor as AnchorInterface<_>>::add_edge(
			tree_id,
			src_chain_id,
			root,
			height,
		));
		let neighbor_index_after: u32 = Anchor::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_eq!(neighbor_index_after, neighbor_index_before + 1);

		for i in 0..(HistoryLength::get() - 1) {
			assert_eq!(
				Anchor::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),
				true
			);

			let val = thread_rng().gen::<[u8; 32]>();
			let elt = Element::from_bytes(&val);
			assert_ok!(<Anchor as AnchorInterface<_>>::update_edge(
				tree_id,
				src_chain_id,
				elt,
				height,
			));

			assert_eq!(
				Anchor::is_known_neighbor_root(tree_id, src_chain_id, elt).unwrap(),
				true
			);
		}

		assert_eq!(
			Anchor::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),
			false
		);
	});
}

#[test]
fn anchor_works() {
	new_test_ext().execute_with(|| {
		const M: usize = 2;
		type Bn254Fr = ark_bn254::Fr;
		let mut rng = ark_std::test_rng();
		let curve = Curve::Bn254;
		let params3 = setup_circom_params_x5_3::<Bn254Fr>(curve);
		let params5 = setup_circom_params_x5_5::<Bn254Fr>(curve);
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		// now let's create the anchor.
		let deposit_size = 10_000;
		let max_edges = 2;
		let depth = LEN as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), deposit_size, max_edges, depth, asset_id));

		// now with anchor created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let src_chain_id = 1;
		let sender_account_id = 1;
		let recipient_account_id = 2;
		let relayer_account_id = 0;
		let fee_value = 0;
		let refund_value = 0;
		// top-up the sender account with some balance.
		let _ = Balances::set_balance(Origin::root(), sender_account_id, 100_000_000, 0);
		// inputs
		let chain_id = Bn254Fr::from(src_chain_id);
		let recipient = Bn254Fr::from(recipient_account_id);
		let relayer = Bn254Fr::from(relayer_account_id);
		let fee = Bn254Fr::from(fee_value);
		let refund = Bn254Fr::from(refund_value);
		let (leaf_private, leaf_public, leaf, nullifier_hash) = setup_leaf_circomx5(chain_id, &params5, &mut rng);
		let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());
		let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
		assert_ok!(Anchor::deposit(
			Origin::signed(sender_account_id),
			tree_id,
			leaf_element
		));
		let tree_root = MerkleTree::get_root(tree_id).unwrap();
		let height = 1;
		// add edges
		assert_ok!(<Anchor as AnchorInterface<_>>::add_edge(
			tree_id,
			src_chain_id,
			tree_root,
			height,
		));
		// check the balance before the withdraw.
		let balance_before = Balances::free_balance(recipient_account_id);

		// now we start to generate the proof.
		let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);
		let (mt, path) = setup_tree_and_create_path_tree_circomx5::<_, LEN>(&[leaf], 0, &params3);
		let root = mt.root().inner();
		let root_element = Element::from_bytes(&root.into_repr().to_bytes_le());
		assert_eq!(root_element, tree_root);

		let mut roots = [Bn254Fr::default(); M];
		roots[0] = root;

		let set_private_inputs = setup_set(&root, &roots);

		let circuit = Circuit_Circomx5::new(
			arbitrary_input,
			leaf_private,
			leaf_public,
			set_private_inputs,
			roots,
			params5,
			path,
			root,
			nullifier_hash,
		);
		let (pk, vk) = setup_groth16_circuit_circomx5::<_, ark_bn254::Bn254, LEN, M>(&mut rng, circuit.clone());

		let mut proving_key_bytes = Vec::new();
		pk.serialize(&mut proving_key_bytes).unwrap();
		let mut verifying_key_bytes = Vec::new();
		vk.serialize(&mut verifying_key_bytes).unwrap();

		let proof = prove_groth16_circuit_circomx5(&pk, circuit, &mut rng);

		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();
		// setup the vk
		let mut verifier_key_bytes = Vec::new();
		vk.serialize(&mut verifier_key_bytes).unwrap();
		assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));
		let roots_element = roots
			.iter()
			.map(|v| Element::from_bytes(&v.into_repr().to_bytes_le()))
			.collect();
		assert_ok!(Anchor::withdraw(
			Origin::signed(sender_account_id),
			tree_id,
			proof_bytes,
			src_chain_id,
			roots_element,
			nullifier_hash_element,
			recipient_account_id,
			relayer_account_id,
			fee_value,
			refund_value,
		));
		// now we check the recipient balance again.
		let balance_after = Balances::free_balance(recipient_account_id);
		assert_eq!(balance_after, balance_before + deposit_size);
		// perfect
	});
}
