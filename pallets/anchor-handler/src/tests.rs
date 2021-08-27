use crate::{mock::*, ChainId, Error};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use pallet_anchor::types::EdgeMetadata;
use pallet_mt::types::ElementTrait;
use sp_runtime::traits::BadOrigin;

use codec::Encode;
use sp_core::{blake2_256, H256};

const TEST_THRESHOLD: u32 = 2;

fn make_anchor_creation_proposal() -> Call {
	let resource_id = [1; 32];
	Call::AnchorHandler(crate::Call::execute_anchor_creation_proposal(resource_id))
}

fn make_anchor_updation_proposal(
	anchor_metadata: EdgeMetadata<
		ChainId<Test>,
		<Test as pallet_mt::Config>::Element,
		<Test as frame_system::Config>::BlockNumber,
	>,
) -> Call {
	let resource_id = [1; 32];
	Call::AnchorHandler(crate::Call::execute_anchor_updatation_proposal(
		resource_id,
		anchor_metadata,
	))
}

#[test]
fn anchor_creation_proposal() {
	new_test_ext().execute_with(|| {
		let src_id = 1;
		// set anchors threshold
		assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
		// add relayers
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
		// whitelish chain
		assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
		// create anchor creation proposal
		let creation_proposal = make_anchor_creation_proposal();
		let prop_id = 1;
		let r_id = pallet_bridge::utils::derive_resource_id(src_id, b"hash");
		let resource = b"AnchorHandler.execute_anchor_creation_proposal".to_vec();
		// set resource id
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));
		// make proposals
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(creation_proposal.clone())
		));
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(creation_proposal.clone())
		));
		// proposal should be voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorCreated);
	})
}

#[test]
fn anchor_creation_updation_proposal() {
	new_test_ext().execute_with(|| {
		let src_id = 1;
		// set anchors threshold
		assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
		// add relayers
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
		// whitelish chain
		assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
		// create anchor creation proposal
		let creation_proposal = make_anchor_creation_proposal();
		let prop_id = 1;
		let r_id = pallet_bridge::utils::derive_resource_id(src_id, b"hash");
		let resource = b"AnchorHandler.execute_anchor_creation_proposal".to_vec();
		// set resource id
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));
		// make proposals
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(creation_proposal.clone())
		));
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(creation_proposal.clone())
		));
		// proposal should be voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorCreated);

		let prop_id = 2;
		let r_id = pallet_bridge::utils::derive_resource_id(src_id, b"hash");
		let resource = b"AnchorHandler.execute_anchor_creation_proposal".to_vec();
		let root = Element::from_bytes(&[1; 32]);
		let height: u64 = 5;
		let edge_metadata = EdgeMetadata {
			src_chain_id: src_id,
			root,
			height,
		};
		// create anchor updation proposal
		let updation_proposal = make_anchor_updation_proposal(edge_metadata);
		// set resource id
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));
		// make proposals
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(updation_proposal.clone())
		));
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(updation_proposal.clone())
		));
		// proposal should be voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorUpdated);
	})
}
