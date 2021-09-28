use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalSerialize;
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	setup::{
		common::{setup_circom_params_x5_3, setup_circom_params_x5_5, setup_tree_and_create_path_tree_circomx5, Curve},
		mixer::{
			prove_groth16_circuit_circomx5, setup_arbitrary_data, setup_groth16_circuit_circomx5, setup_leaf_circomx5,
			Circuit_Circomx5,
		},
	},
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use darkwebb_primitives::{merkle_tree::TreeInspector, ElementTrait};
use frame_support::{assert_ok, traits::OnInitialize};
use sp_runtime::traits::One;

use crate::mock::*;

const LEN: usize = 30;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

#[test]
fn should_create_new_mixer() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3, 0));
	});
}

#[test]
fn should_be_able_to_deposit() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 3, 0));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let account_id = 1;
		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id);
		// and we do the deposit
		assert_ok!(Mixer::deposit(Origin::signed(account_id), tree_id, leaf));
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
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3, 0));
		let default_maintainer_account_id = 0;
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);
		let new_maintainer_account_id = 1;
		assert_ok!(Mixer::force_set_maintainer(Origin::root(), new_maintainer_account_id));
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}

#[test]
fn mixer_works() {
	new_test_ext().execute_with(|| {
		type Bn254Fr = ark_bn254::Fr;
		let mut rng = ark_std::test_rng();
		let curve = Curve::Bn254;
		let params3 = setup_circom_params_x5_3::<Bn254Fr>(curve);
		let params5 = setup_circom_params_x5_5::<Bn254Fr>(curve);
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		// now let's create the mixer.
		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = 1;
		let recipient_account_id = 2;
		let relayer_account_id = 0;
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient = Bn254Fr::from(recipient_account_id);
		let relayer = Bn254Fr::from(relayer_account_id);
		let fee = Bn254Fr::from(fee_value);
		let refund = Bn254Fr::from(refund_value);
		let (leaf_private, leaf, nullifier_hash) = setup_leaf_circomx5(&params5, &mut rng);
		let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());
		let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
		assert_ok!(Mixer::deposit(Origin::signed(sender_account_id), tree_id, leaf_element,));
		// check the balance before the withdraw.
		let balance_before = Balances::free_balance(recipient_account_id);

		// now we start to generate the proof.
		let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);
		let (mt, path) = setup_tree_and_create_path_tree_circomx5(&[leaf], 0, &params3);
		let root = mt.root().inner();
		let root_element = Element::from_bytes(&root.into_repr().to_bytes_le());
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		let circuit = Circuit_Circomx5::new(arbitrary_input, leaf_private, (), params5, path, root, nullifier_hash);
		let (pk, vk) = setup_groth16_circuit_circomx5::<_, ark_bn254::Bn254, LEN>(&mut rng, circuit.clone());
		let proof = prove_groth16_circuit_circomx5::<_, ark_bn254::Bn254, LEN>(&pk, circuit, &mut rng);
		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();
		// setup the vk
		let mut verifier_key_bytes = Vec::new();
		vk.serialize(&mut verifier_key_bytes).unwrap();
		assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));
		assert_ok!(Mixer::withdraw(
			Origin::signed(sender_account_id),
			tree_id,
			proof_bytes,
			root_element,
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
