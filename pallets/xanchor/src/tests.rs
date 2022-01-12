use super::{
	mock::{parachain::*, *},
	test_utils::*,
	*,
};
use std::{convert::TryInto, path::Path};
use ark_bn254::Bn254;
use frame_benchmarking::account;
use arkworks_utils::utils::common::{Curve, setup_params_x5_4, setup_params_x5_3};
use ark_bn254::Fr as Bn254Fr;
use codec::Encode;
use webb_primitives::utils::encode_resource_id;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use pallet_anchor::BalanceOf;
use pallet_democracy::{AccountVote, Conviction, Vote};
use xcm_simulator::TestExt;
use arkworks_circuits::setup::common::setup_keys;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;
const DEPOSIT_SIZE: u128 = 10_000;

fn setup_environment(curve: Curve) -> Vec<u8> {
	match curve {
		Curve::Bn254 => {
			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params4 = setup_params_x5_4::<Bn254Fr>(curve);

			// 1. Setup The Hasher Pallet.
			assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
			// 2. Initialize MerkleTree pallet.
			<MerkleTree as OnInitialize<u64>>::on_initialize(1);
			// 3. Setup the VerifierPallet
			//    but to do so, we need to have a VerifyingKey
			let (pk_bytes, vk_bytes) = if Path::new("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5_4_leaf/proving_key.bin").exists() {
				let (pk, vk) = (
					std::fs::read("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5_4_leaf/proving_key.bin").expect("Unable to read file").to_vec(),
					std::fs::read("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5_4_leaf/verifying_key.bin").expect("Unable to read file").to_vec()
				);
				(pk.to_vec(), vk.to_vec())
			} else {
				let rng = &mut ark_std::test_rng();
				let anchor_setup = AnchorSetup30_2::new(params3, params4);
				let (circuit, .., public_inputs) = anchor_setup.setup_random_circuit(rng).unwrap();
				let (pk, vk) = setup_keys::<Bn254, _, _>(circuit.clone(), rng).unwrap();
				std::fs::write("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5_4_leaf/proving_key.bin", &pk).expect("Unable to write file");
				std::fs::write("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5_4_leaf/verifying_key.bin", &vk).expect("Unable to write file");
				(pk, vk)
			};

			assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), vk_bytes.to_vec()));

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
			pk_bytes.to_vec()
		}
		Curve::Bls381 => {
			unimplemented!()
		}
	}
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
		let account_id = parachain::AccountOne::get();
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
			XAnchor::register_resource_id(Origin::signed(parachain::AccountTwo::get()), r_id, target_tree_id),
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
			XAnchor::update(Origin::signed(parachain::AccountTwo::get()), r_id, Default::default()),
			frame_support::error::BadOrigin,
		);
	});
}

// Governance System Tests
fn aye(who: AccountId) -> AccountVote<BalanceOf<Runtime, ()>> {
	AccountVote::Standard {
		vote: Vote {
			aye: true,
			conviction: Conviction::None,
		},
		balance: Balances::free_balance(&who),
	}
}

fn nay(who: AccountId) -> AccountVote<BalanceOf<Runtime, ()>> {
	AccountVote::Standard {
		vote: Vote {
			aye: false,
			conviction: Conviction::None,
		},
		balance: Balances::free_balance(&who),
	}
}

#[test]
fn governance_system_works() {
	MockNet::reset();
	// create an anchor on parachain A.
	let para_a_tree_id = ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});
	// Also, Create an anchor on parachain B.
	let para_b_tree_id = ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// next, we start doing the linking process through the governance system.
	ParaA::execute_with(|| {
		// create a link proposal, saying that we (parachain A) want to link the anchor
		// (local_tree_id) to the anchor (target_tree_id) located on Parachain B
		// (target_chain_id).
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_b_tree_id),
			local_tree_id: para_a_tree_id,
		};
		let value = 100;
		assert_ok!(XAnchor::propose_to_link_anchor(
			Origin::signed(AccountThree::get()),
			payload,
			value
		));
		// we should see this anchor link in the pending list
		assert_eq!(
			XAnchor::pending_linked_anchors(PARAID_B, para_a_tree_id),
			Some(para_b_tree_id),
		);

		// start of 2 => next referendum scheduled.
		fast_forward_to(2);
		// now we need to vote on the proposal.
		let referendum_index = Democracy::referendum_count() - 1;
		assert_ok!(Democracy::vote(
			Origin::signed(AccountOne::get()),
			referendum_index,
			aye(AccountOne::get())
		));
		// referendum runs during 2 and 3, ends @ start of 4.
		fast_forward_to(4);
		// referendum passes and wait another two blocks for enactment.
		fast_forward_to(6);
		// at this point the proposal should be enacted and we sent a message to
		// the other chain.
	});

	// now we do the on-chain proposal checking on chain B.
	ParaB::execute_with(|| {
		// we should see the anchor in the pending list.
		assert_eq!(
			XAnchor::pending_linked_anchors(PARAID_A, para_b_tree_id),
			Some(para_a_tree_id),
		);
		// start of 2 => next referendum scheduled.
		fast_forward_to(2);
		// now we need to vote on the proposal.
		let referendum_index = Democracy::referendum_count() - 1;
		assert_ok!(Democracy::vote(
			Origin::signed(AccountTwo::get()),
			referendum_index,
			aye(AccountTwo::get())
		));
		// referendum runs during 4 and 5, ends @ start of 6.
		fast_forward_to(4);
		// referendum passes and wait another two blocks for enactment.
		fast_forward_to(6);
		// at this point the proposal should be enacted and the anchors should be linked
		// on this chain.
		assert_eq!(XAnchor::pending_linked_anchors(PARAID_A, para_b_tree_id), None,);
		assert_eq!(XAnchor::linked_anchors(PARAID_A, para_b_tree_id), para_a_tree_id);
	});

	// on chain A we should find them linked too.
	ParaA::execute_with(|| {
		assert_eq!(XAnchor::pending_linked_anchors(PARAID_B, para_a_tree_id), None);
		assert_eq!(XAnchor::linked_anchors(PARAID_B, para_a_tree_id), para_b_tree_id);
	});

	// the link process is now done!
}

// Some negtive tests for the governance system.

#[test]
fn should_fail_to_create_proposal_if_the_anchor_does_not_exist() {
	MockNet::reset();
	// creating a proposal for a non-existing anchor.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(MerkleTree::next_tree_id()),
			local_tree_id: MerkleTree::next_tree_id(),
		};
		let value = 100;
		assert_err!(
			XAnchor::propose_to_link_anchor(Origin::signed(AccountThree::get()), payload, value),
			Error::<Runtime>::AnchorNotFound
		);
	});
}

#[test]
fn should_fail_to_create_proposal_for_already_linked_anchors() {
	MockNet::reset();
	// create an anchor on parachain A.
	let para_a_tree_id = ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});
	// create an anchor on parachain B.
	let para_b_tree_id = ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// force link them.
	ParaA::execute_with(|| {
		let r_id = encode_resource_id(para_a_tree_id, PARAID_B);
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_b_tree_id
		));
	});

	// now try create a proposal on chain A, it should fail since it is already
	// linked.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_b_tree_id),
			local_tree_id: para_a_tree_id,
		};
		let value = 100;
		assert_err!(
			XAnchor::propose_to_link_anchor(Origin::signed(AccountThree::get()), payload, value),
			Error::<Runtime>::ResourceIsAlreadyAnchored
		);
	});
}

#[test]
fn should_fail_to_create_proposal_for_already_pending_linking() {
	MockNet::reset();
	// create an anchor on parachain A.
	let para_a_tree_id = ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// create an anchor on parachain B.
	let para_b_tree_id = ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// create a proposal on chain A.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_b_tree_id),
			local_tree_id: para_a_tree_id,
		};
		let value = 100;
		assert_ok!(XAnchor::propose_to_link_anchor(
			Origin::signed(AccountThree::get()),
			payload,
			value
		));
	});

	// now try create a new proposal on chain A with different Account it should
	// fail.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_b_tree_id),
			local_tree_id: para_a_tree_id,
		};
		let value = 100;
		assert_err!(
			XAnchor::propose_to_link_anchor(Origin::signed(AccountFour::get()), payload, value),
			Error::<Runtime>::AnchorLinkIsAlreadyPending
		);
	});
}

#[test]
fn should_fail_to_call_send_link_anchor_message_as_signed_account() {
	// reset the network.
	MockNet::reset();
	// calling send_link_anchor_message as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(MerkleTree::next_tree_id()),
			local_tree_id: MerkleTree::next_tree_id(),
		};
		let value = 100;
		assert_err!(
			XAnchor::send_link_anchor_message(Origin::signed(AccountThree::get()), payload, value),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_call_save_link_proposal_as_signed_account() {
	// rest the network.
	MockNet::reset();
	// calling save_link_proposal as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(MerkleTree::next_tree_id()),
			local_tree_id: MerkleTree::next_tree_id(),
		};
		assert_err!(
			XAnchor::save_link_proposal(Origin::signed(AccountThree::get()), payload,),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_save_link_proposal_on_already_linked_anchors() {
	// rest the network.
	MockNet::reset();
	// create an anchor on parachain A.
	let para_a_tree_id = ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// create an anchor on parachain B.
	let para_b_tree_id = ParaB::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});

	// force link them.
	ParaA::execute_with(|| {
		let r_id = encode_resource_id(para_a_tree_id, PARAID_B);
		assert_ok!(XAnchor::force_register_resource_id(
			Origin::root(),
			r_id,
			para_b_tree_id
		));
	});

	// now creating a proposal on chain A should fail since it is already linked.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_b_tree_id),
			local_tree_id: para_a_tree_id,
		};
		let value = 100;
		assert_err!(
			XAnchor::propose_to_link_anchor(Origin::signed(AccountThree::get()), payload, value),
			Error::<Runtime>::ResourceIsAlreadyAnchored
		);
	});
}

#[test]
fn should_fail_to_call_handle_link_anchor_message_without_anchor_being_pending() {
	// reset the network.
	MockNet::reset();
	// create an anchor on parachain A.
	let para_a_tree_id = ParaA::execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		MerkleTree::next_tree_id() - 1
	});
	// now calling handle_link_anchor_message directly should fail.
	// on parachain A, since this anchor is not pending.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(para_a_tree_id),
			local_tree_id: 0,
		};
		let value = 100;
		assert_err!(
			XAnchor::handle_link_anchor_message(Origin::signed(AccountThree::get()), payload, value),
			Error::<Runtime>::AnchorLinkNotFound,
		);
	});
}

#[test]
fn should_fail_to_call_link_anchors_as_signed_account() {
	// reset the network.
	MockNet::reset();
	// calling link_anchors as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(MerkleTree::next_tree_id()),
			local_tree_id: MerkleTree::next_tree_id(),
		};
		assert_err!(
			XAnchor::link_anchors(Origin::signed(AccountThree::get()), payload),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_call_handle_link_anchors_as_signed_account() {
	// reset the network.
	MockNet::reset();
	// calling handle_link_anchors as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let payload = LinkProposal {
			target_chain_id: PARAID_B,
			target_tree_id: Some(MerkleTree::next_tree_id()),
			local_tree_id: MerkleTree::next_tree_id(),
		};
		assert_err!(
			XAnchor::handle_link_anchors(Origin::signed(AccountThree::get()), payload),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_call_register_resource_id_as_signed_account() {
	// reset the network.
	MockNet::reset();
	// calling register_resource_id as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let r_id = encode_resource_id(MerkleTree::next_tree_id(), PARAID_B);
		assert_err!(
			XAnchor::register_resource_id(Origin::signed(AccountThree::get()), r_id, MerkleTree::next_tree_id()),
			frame_support::error::BadOrigin,
		);
	});
}

#[test]
fn should_fail_to_call_update_as_signed_account() {
	// reset the network.
	MockNet::reset();
	// calling update as signed account should fail.
	// on parachain A.
	ParaA::execute_with(|| {
		let r_id = encode_resource_id(MerkleTree::next_tree_id(), PARAID_B);
		let edge_metadata = EdgeMetadata {
			src_chain_id: PARAID_B,
			root: Element::zero(),
			latest_leaf_index: 0,
		};
		assert_err!(
			XAnchor::update(Origin::signed(AccountThree::get()), r_id, edge_metadata),
			frame_support::error::BadOrigin,
		);
	});
}
