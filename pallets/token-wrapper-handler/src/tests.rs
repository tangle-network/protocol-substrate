use std::convert::TryInto;

use crate::mock::*;

use asset_registry::AssetType;
use frame_support::{assert_err, assert_ok, error::BadOrigin};
use pallet_bridge::types::{ProposalStatus, ProposalVotes};

const TEST_THRESHOLD: u32 = 2;

fn make_wrapping_fee_proposal(resource_id: &[u8; 32], wrapping_fee_percent: u128) -> Call {
	Call::TokenWrapperHandler(crate::Call::execute_wrapping_fee_proposal {
		r_id: *resource_id,
		wrapping_fee_percent,
	})
}

fn setup_relayers(src_id: u32) {
	// set anchors threshold
	assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
	// add relayers
	assert_eq!(Bridge::relayer_count(), 0);
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
	assert_eq!(Bridge::relayer_count(), 2);
	// whitelist chain
	assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
}

fn relay_fee_update_proposal(src_chain_id: u32, resource_id: &[u8; 32], prop_id: u64, wrapping_fee_percent: u128) {
	// create fee update proposal
	let resource = b"TokenWrapperHandler.execute_wrapping_fee_proposal".to_vec();
	let update_proposal = make_wrapping_fee_proposal(resource_id, wrapping_fee_percent);
	// set resource id
	assert_ok!(Bridge::set_resource(Origin::root(), *resource_id, resource));
	// make proposals
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_A),
		prop_id,
		src_chain_id,
		*resource_id,
		Box::new(update_proposal.clone())
	));
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_B),
		prop_id,
		src_chain_id,
		*resource_id,
		Box::new(update_proposal)
	));
}

#[test]
fn should_update_fee() {
	new_test_ext().execute_with(|| {
		let src_chain_id = 1;
		let resource_id = pallet_bridge::utils::derive_resource_id(src_chain_id, b"hash");
		let prop_id = 1;
		// create fee update proposal
		setup_relayers(src_chain_id);
		relay_fee_update_proposal(src_chain_id, &resource_id, prop_id, 5);
		assert_eq!(TokenWrapper::get_wrapping_fee(1000_u128), 52);
	})
}
