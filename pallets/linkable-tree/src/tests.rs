use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	merkle_tree::TreeInspector,
	AccountId, ElementTrait,
};

use codec::Encode;

use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin, traits::OnInitialize};

use crate::mock::*;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;

fn setup_environment(curve: Curve) -> Vec<u8> {
	let rng = &mut ark_std::test_rng();
	let params = match curve {
		Curve::Bn254 => {
			let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
			let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
			PoseidonParameters::new(rounds, mds)
		}
		Curve::Bls381 => todo!("Setup environment for bls381"),
	};
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey
	let mut verifier_key_bytes = Vec::new();
	let mut proving_key_bytes = Vec::new();

	match curve {
		Curve::Bn254 => {
			let (pk, vk) = setup_groth16_random_circuit_circomx5::<_, ark_bn254::Bn254, TREE_DEPTH, M>(rng, curve);
			vk.serialize(&mut verifier_key_bytes).unwrap();
			pk.serialize(&mut proving_key_bytes).unwrap();
		}
		Curve::Bls381 => {
			let (pk, vk) =
				setup_groth16_random_circuit_circomx5::<_, ark_bls12_381::Bls12_381, TREE_DEPTH, M>(rng, curve);
			vk.serialize(&mut verifier_key_bytes).unwrap();
			pk.serialize(&mut proving_key_bytes).unwrap();
		}
	};
	assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));
	// 4. and top-up some accounts with some balance
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
		account::<AccountId>("", 6, SEED),
	] {
		assert_ok!(Balances::set_balance(Origin::root(), account_id, 100_000_000, 0));
	}
	// finally return the provingkey bytes
	proving_key_bytes
}

#[test]
fn should_create_new_anchor() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
	});
}

#[test]
fn should_fail_to_create_new_anchor_if_not_root() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_err!(
			Anchor::create(
				Origin::signed(account::<AccountId>("", 1, SEED)),
				DEPOSIT_SIZE,
				max_edges,
				depth,
				asset_id
			),
			BadOrigin,
		);
	});
}

#[test]
fn should_be_able_to_change_the_maintainer() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));

		let default_maintainer_account_id = AccountId::default();
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);

		let new_maintainer_account_id = account::<AccountId>("", 1, SEED);
		assert_ok!(Anchor::force_set_maintainer(
			Origin::root(),
			new_maintainer_account_id.clone()
		));
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}

#[test]
fn should_fail_to_change_the_maintainer_if_not_the_current_maintainer() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		let current_maintainer_account_id = Anchor::maintainer();
		let new_maintainer_account_id = account::<AccountId>("", 1, SEED);
		assert_err!(
			Anchor::set_maintainer(
				Origin::signed(account::<AccountId>("", 2, SEED)),
				new_maintainer_account_id
			),
			crate::Error::<Test, _>::InvalidPermissions,
		);
		// maintainer should never be changed.
		assert_eq!(current_maintainer_account_id, Anchor::maintainer());
	});
}

#[test]
fn should_be_able_to_add_neighbors_and_check_history() {
	use rand::prelude::*;

	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);

		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
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

		for _ in 0..(HistoryLength::get() - 1) {
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

fn create_anchor() -> u32 {
	let max_edges = 2;
	let depth = TREE_DEPTH as u8;
	let asset_id = 0;
	assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}
