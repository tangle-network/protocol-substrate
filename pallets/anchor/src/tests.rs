use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
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
		assert_ok!(<Anchor as AnchorInterface<_, _, _, _, _, _, _>>::add_edge(
			tree_id,
			src_chain_id,
			root,
			height,
		));
		let neighbor_index_after: u32 = Anchor::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_eq!(neighbor_index_after, neighbor_index_before + 1);

		for i in 0..(crate::mock::HistoryLength::get() - 1) {
			assert_eq!(
				Anchor::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),
				true
			);

			let val = thread_rng().gen::<[u8; 32]>();
			let elt = Element::from_bytes(&val);
			assert_ok!(<Anchor as AnchorInterface<_, _, _, _, _, _, _>>::update_edge(
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
