use super::{
	mock::{parachain::*, *},
	test_utils::*,
	*,
};

use arkworks_gadgets::setup::common::Curve;
use codec::Encode;
use darkwebb_primitives::utils::encode_resource_id;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use xcm_simulator::TestExt;

const TREE_DEPTH: usize = 30;
const M: usize = 2;
const DEPOSIT_SIZE: u128 = 10_000;

fn setup_environment(curve: Curve) -> Vec<u8> {
	let params = match curve {
		Curve::Bn254 => get_hash_params::<ark_bn254::Fr>(curve),
		Curve::Bls381 => {
			todo!("Setup hash params for bls381")
		}
	};
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params.0));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey
	let mut verifier_key_bytes = Vec::new();
	let mut proving_key_bytes = Vec::new();

	get_keys(curve, &mut proving_key_bytes, &mut verifier_key_bytes);

	assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));

	// finally return the provingkey bytes
	proving_key_bytes
}

// sanity check that XCM is working
#[test]
fn dmp() {
	MockNet::reset();

	let remark =
		parachain::Call::System(frame_system::Call::<parachain::Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	Relay::execute_with(|| {
		assert_ok!(RelayChainPalletXcm::send_xcm(
			Here,
			Parachain(PARAID_A),
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	ParaA::execute_with(|| {
		use parachain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn ump() {
	MockNet::reset();

	let remark = relay_chain::Call::System(frame_system::Call::<relay_chain::Runtime>::remark_with_event {
		remark: vec![1, 2, 3],
	});
	ParaA::execute_with(|| {
		assert_ok!(ParachainPalletXcm::send_xcm(
			Here,
			Parent,
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	Relay::execute_with(|| {
		use relay_chain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn xcmp() {
	MockNet::reset();

	let remark =
		parachain::Call::System(frame_system::Call::<parachain::Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	ParaA::execute_with(|| {
		assert_ok!(ParachainPalletXcm::send_xcm(
			Here,
			(Parent, Parachain(PARAID_B)),
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	ParaB::execute_with(|| {
		use parachain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn should_link_two_anchors() {
	MockNet::reset();
	let mut para_a_tree_id = 0;
	let mut para_b_tree_id = 0;

	ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		para_a_tree_id = MerkleTree::next_tree_id() - 1;
	});

	ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		para_b_tree_id = MerkleTree::next_tree_id() - 1;
	});

	// The caller her is one of the Parachain B operators.
	// it will try to link the para_a_tree_id to para_b_tree_id
	// we tell ParaA to link the `para_a_tree_id` to `para_b_tree_id` on the ParaB.
	ParaA::execute_with(|| {
		// the resource id reads as following
		// we need to link para_a_tree_id to another anchor defined on ParaB
		let r_id = encode_resource_id(para_a_tree_id, PARAID_B);
		// then, on the call here, we tell it which tree we are going to link to.
		// (para_b_tree_id).
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_b_tree_id
		));
	});
	// Here, the same as above, but the only difference is that
	// the caller is one of the Parachain A operators.
	ParaB::execute_with(|| {
		// we need to link para_b_tree_id to another anchor defined on ParaA
		let r_id = encode_resource_id(para_b_tree_id, PARAID_A);
		// then, when we are sending the call we tell it which tree we are going to link
		// to. (para_a_tree_id).
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_a_tree_id
		));
	});

	// now we assume both of them are linked, let's check that.
	ParaA::execute_with(|| {
		let exists =
			crate::LinkedAnchors::<parachain::Runtime, _>::iter().any(|(chain_id, tree_id, target_tree_id)| {
				chain_id == PARAID_B && tree_id == para_a_tree_id && target_tree_id == para_b_tree_id
			});
		assert!(exists, "ParaA does not have link to ParaB");
	});

	ParaB::execute_with(|| {
		let exists =
			crate::LinkedAnchors::<parachain::Runtime, _>::iter().any(|(chain_id, tree_id, target_tree_id)| {
				chain_id == PARAID_A && tree_id == para_b_tree_id && target_tree_id == para_a_tree_id
			});
		assert!(exists, "ParaB does not have link to ParaB");
	});
}

#[test]
fn should_bridge_anchors_using_xcm() {
	MockNet::reset();
	let mut para_a_tree_id = 0;
	let mut para_b_tree_id = 0;

	ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		para_a_tree_id = MerkleTree::next_tree_id() - 1;
	});

	ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		para_b_tree_id = MerkleTree::next_tree_id() - 1;
	});

	ParaA::execute_with(|| {
		let r_id = encode_resource_id(para_a_tree_id, PARAID_B);
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_b_tree_id
		));
	});

	ParaB::execute_with(|| {
		let r_id = encode_resource_id(para_b_tree_id, PARAID_A);
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_a_tree_id
		));
	});

	// now we do a deposit on one chain (ParaA) for example
	// and check the edges on the other chain (ParaB).
	let mut para_a_root = Element::from_bytes(&[0u8; 32]);
	ParaA::execute_with(|| {
		let account_id = ALICE;
		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id.clone());
		// and we do the deposit
		assert_ok!(Anchor::deposit_and_update_linked_anchors(
			Origin::signed(account_id.clone()),
			para_a_tree_id,
			leaf
		));
		// now we check the balance after the deposit.
		let balance_after = Balances::free_balance(account_id);
		// the balance should be less now with `deposit_size`
		assert_eq!(balance_after, balance_before - DEPOSIT_SIZE);
		// now we need also to check if the state got updated.
		let tree = MerkleTree::trees(para_a_tree_id);
		assert_eq!(tree.leaf_count, 1);
		para_a_root = tree.root;
	});

	// ok now we go to ParaB and check the edges.
	// we should expect that the edge for ParaA is there, and the merkle root equal
	// to the one we got from ParaA.
	ParaB::execute_with(|| {
		let edge = LinkableTree::edge_list(para_b_tree_id, PARAID_A);
		assert_eq!(edge.root, para_a_root);
		assert_eq!(edge.latest_leaf_index, 1);
	});
	// Nice!
}

#[test]
fn should_fail_to_register_resource_id_if_not_the_democracy() {
	MockNet::reset();
	// it should fail to register a resource id if not the current maintainer.
	ParaA::execute_with(|| {
		let tree_id = MerkleTree::next_tree_id() - 1;
		let r_id = encode_resource_id(tree_id, PARAID_B);
		let target_tree_id = 1;
		assert_err!(
			XAnchor::register_resource_id(Origin::signed(BOB), r_id, target_tree_id),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_register_resource_id_when_anchor_deos_not_exist() {
	MockNet::reset();
	// it should fail to register the resource id if the anchor does not exist.
	ParaA::execute_with(|| {
		// anchor/tree does not exist.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let r_id = encode_resource_id(tree_id, PARAID_B);
		let target_tree_id = 1;
		assert_err!(
			XAnchor::register_resource_id(Origin::root(), r_id, target_tree_id),
			crate::Error::<parachain::Runtime, _>::AnchorNotFound,
		);
	});
}

#[test]
fn should_fail_to_link_anchor_if_it_is_already_anchored() {
	// it should fail if the resource id is already anchored.
	MockNet::reset();
	ParaA::execute_with(|| {
		// first we create the anchor
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		// next we start to register the resource id.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let r_id = encode_resource_id(tree_id, PARAID_B);
		let target_tree_id = 1;
		assert_ok!(XAnchor::register_resource_id(Origin::root(), r_id, target_tree_id));
		// now we try to link the anchor again, should error.
		assert_err!(
			XAnchor::register_resource_id(Origin::root(), r_id, target_tree_id),
			crate::Error::<parachain::Runtime, _>::ResourceIsAlreadyAnchored
		);
	});
}

#[test]
fn ensure_that_the_only_way_to_update_edges_is_from_another_parachain() {
	// in this test we need to ensure that the only way you can call `update` is
	// from another parachain.
	MockNet::reset();
	ParaA::execute_with(|| {
		// try to update the edges, from a normal account!
		// it should fail.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let r_id = encode_resource_id(tree_id, PARAID_B);
		assert_err!(
			XAnchor::update(Origin::signed(BOB), r_id, Default::default()),
			frame_support::error::BadOrigin,
		);
	});
}
