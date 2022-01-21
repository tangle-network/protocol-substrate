#![cfg(test)]

use super::{
	mock::{
		assert_events, new_test_ext, Balances, Bridge, Call, ChainIdentifier, Event, Origin,
		ProposalLifetime, System, Test, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
		TEST_THRESHOLD,
	},
	*,
};
use crate::{
	mock::new_test_ext_initialized,
	{self as pallet_bridge},
};
use frame_support::{assert_err, assert_noop, assert_ok};
use hex_literal::hex;
use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair, Public,
};

use crate::utils::derive_resource_id;

// const SEED: String =
// "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
// const PUBLIC: String =
// "8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
// ;

#[test]
fn derive_ids() {
	let chain: u32 = 0xaabbccdd;
	let id = [
		0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0, 0x24,
		0xb7, 0xb1, 0x09, 0x99, 0xf4,
	];
	let r_id = derive_resource_id(chain, &id);
	let expected = [
		0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a,
		0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0, 0x24, 0xb7, 0xb1, 0x09, 0x99, 0xf4, 0xdd, 0xcc, 0xbb,
		0xaa,
	];
	assert_eq!(r_id, expected);
}

#[test]
fn setup_resources() {
	new_test_ext().execute_with(|| {
		let id: ResourceId = [1; 32];
		let method = "Pallet.do_something".as_bytes().to_vec();
		let method2 = "Pallet.do_somethingElse".as_bytes().to_vec();

		assert_ok!(Bridge::set_resource(Origin::root(), id, method.clone()));
		assert_eq!(Bridge::resources(id), Some(method));
		assert_ok!(Bridge::set_resource(Origin::root(), id, method2.clone()));
		assert_eq!(Bridge::resources(id), Some(method2));

		assert_ok!(Bridge::remove_resource(Origin::root(), id));
		assert_eq!(Bridge::resources(id), None);
	})
}

#[test]
fn whitelist_chain() {
	new_test_ext().execute_with(|| {
		assert!(!Bridge::chain_whitelisted(0));

		assert_ok!(Bridge::whitelist_chain(Origin::root(), 0));
		assert_noop!(
			Bridge::whitelist_chain(Origin::root(), ChainIdentifier::get()),
			Error::<Test>::InvalidChainId
		);

		assert_events(vec![Event::Bridge(pallet_bridge::Event::ChainWhitelisted { chain_id: 0 })]);
	})
}

fn make_proposal(r: Vec<u8>) -> mock::Call {
	Call::System(system::Call::remark { remark: r })
}

#[test]
fn create_proposal_tests() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"remark");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_proposal(vec![10]);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();
		// should fail to execute proposal as non-maintainer
		assert_err!(
			Bridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			),
			Error::<Test>::InvalidPermissions
		);

		// set the new maintainer
		assert_ok!(Bridge::force_set_maintainer(Origin::root(), public_uncompressed.to_vec()));
		// Create proposal (& vote)
		assert_ok!(Bridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		assert_events(vec![
			Event::Bridge(pallet_bridge::Event::ProposalApproved {
				chain_id: src_id,
				proposal_nonce: prop_id,
			}),
			Event::Bridge(pallet_bridge::Event::ProposalSucceeded {
				chain_id: src_id,
				proposal_nonce: prop_id,
			}),
		]);
	})
}
