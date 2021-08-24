use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use frame_support::{assert_ok, traits::OnInitialize};
use pallet_mt::types::ElementTrait;
use sp_runtime::traits::One;

use crate::mock::*;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

#[test]
fn shout_create_new_mixer() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3));
	});
}

#[test]
fn shout_be_able_to_deposit() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 3));
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
fn shout_be_able_to_change_the_maintainer() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3));
		let default_maintainer_account_id = 0;
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);
		let new_maintainer_account_id = 1;
		assert_ok!(Mixer::force_set_maintainer(Origin::root(), new_maintainer_account_id));
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}
