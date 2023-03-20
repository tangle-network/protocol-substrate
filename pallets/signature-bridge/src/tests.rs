#![cfg(test)]

use super::{
	mock::{
		assert_events, new_test_ext, Bridge, RuntimeCall, RuntimeEvent, RuntimeOrigin, Test,
		RELAYER_A,
	},
	*,
};
use crate::{
	mock::new_test_ext_initialized,
	{self as pallet_bridge},
};
use frame_support::{assert_err, assert_ok, BoundedVec};
use hex_literal::hex;
use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair,
};
use sp_runtime::traits::ConstU32;
use webb_primitives::{
	utils::{compute_chain_id_type, derive_resource_id},
	webb_proposals::SubstrateTargetSystem,
};

#[test]
fn setup_resources() {
	new_test_ext().execute_with(|| {
		let id: ResourceId = ResourceId([1; 32]);
		assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), id));
		assert_eq!(Bridge::resources(id), Some(()));
		assert_ok!(Bridge::remove_resource(RuntimeOrigin::root(), id));
		assert_eq!(Bridge::resources(id), None);
	})
}

fn make_proposal(r: Vec<u8>) -> mock::RuntimeCall {
	mock::RuntimeCall::System(system::Call::remark { remark: r })
}

fn make_proposal_data(encoded_r_id: Vec<u8>, nonce: [u8; 4], encoded_call: Vec<u8>) -> Vec<u8> {
	let mut prop_data = encoded_r_id;
	prop_data.extend_from_slice(&[0u8; 4]);
	prop_data.extend_from_slice(&nonce);
	prop_data.extend_from_slice(&encoded_call[..]);
	prop_data
}

#[test]
fn create_proposal_tests() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let r_id = derive_resource_id(1080u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 });
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let call = make_proposal(vec![10]);
		let call_encoded = call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);

		// should fail to execute proposal as non-maintainer
		assert_err!(
			Bridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id,
				prop_data.clone().try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			Error::<Test>::InvalidPermissions
		);

		// set the new maintainer
		assert_ok!(Bridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));
		// Create proposal (& vote)
		assert_ok!(Bridge::execute_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			src_id,
			prop_data.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));

		assert_events(vec![
			RuntimeEvent::Bridge(pallet_bridge::Event::ProposalApproved {
				chain_id: src_id,
				proposal_nonce: u32::from_be_bytes(nonce),
			}),
			RuntimeEvent::Bridge(pallet_bridge::Event::ProposalSucceeded {
				chain_id: src_id,
				proposal_nonce: u32::from_be_bytes(nonce),
			}),
		]);
	})
}

// Nonce Tests
// #[test]
// fn should_fail_to_set_resource_id_with_same_nonce() {
// 	let chain_type = [2, 0];
// 	let src_id = compute_chain_id_type(1u32, chain_type);
// 	let r_id = derive_resource_id(1080u32, SubstrateTargetSystem { pallet_index: 2,
// tree_id: 1 }).into(); 	let public_uncompressed =
// hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
// ); 	let pair = ecdsa::Pair::from_string(
// 		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
// 		None,
// 	)
// 	.unwrap();

// 	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
// 		let call = make_proposal(vec![10]);
// 		let call_encoded = call.encode();
// 		let nonce = [0u8, 0u8, 0u8, 1u8];
// 		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
// 		let msg = keccak_256(&prop_data);
// 		let sig: Signature = pair.sign_prehashed(&msg).into();

// 		// set the new maintainer
// 		assert_ok!(Bridge::force_set_maintainer(RuntimeOrigin::root(), public_uncompressed.to_vec()));
// 		// Create proposal (& vote)
// 		assert_ok!(Bridge::set_resource_with_signature(
// 			RuntimeOrigin::signed(RELAYER_A),
// 			src_id,
// 			Box::new(call.clone()),
// 			prop_data.clone(),
// 			sig.0.to_vec(),
// 		));

// 		assert_events(vec![
// 			Event::Bridge(pallet_bridge::Event::ProposalApproved {
// 				chain_id: src_id,
// 				proposal_nonce: u32::from_be_bytes(nonce),
// 			}),
// 			Event::Bridge(pallet_bridge::Event::ProposalSucceeded {
// 				chain_id: src_id,
// 				proposal_nonce: u32::from_be_bytes(nonce),
// 			}),
// 		]);

// 		let call = make_proposal(vec![10]);
// 		let call_encoded = call.encode();
// 		let nonce = [0u8, 0u8, 0u8, 1u8];
// 		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
// 		let msg = keccak_256(&prop_data);
// 		let sig: Signature = pair.sign_prehashed(&msg).into();

// 		assert_err!(
// 			Bridge::set_resource_with_signature(
// 				RuntimeOrigin::signed(RELAYER_A),
// 				src_id,
// 				Box::new(call.clone()),
// 				prop_data.clone(),
// 				sig.0.to_vec(),
// 			),
// 			Error::<Test>::InvalidNonce
// 		);
// 	})
// }

#[test]
fn should_fail_to_set_resource_id_when_nonce_increments_by_more_than_1048() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let r_id = derive_resource_id(5u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 });
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let call = make_proposal(vec![10]);
		let call_encoded = call.encode();
		let nonce = [0u8, 0u8, 4u8, 120u8];
		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);

		assert_err!(
			Bridge::set_resource_with_signature(
				RuntimeOrigin::signed(RELAYER_A),
				src_id,
				prop_data.try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			Error::<Test>::InvalidNonce
		);
	})
}

#[test]
fn set_maintainer_should_work() {
	let r_id = derive_resource_id(5u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 });
	let new_maintainer = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");

	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();
	let old_maintainer =
		libsecp256k1::PublicKey::parse_compressed(&pair.public().0).unwrap().serialize()[1..]
			.to_vec();

	new_test_ext_initialized(1u64, r_id, b"System.remark".to_vec()).execute_with(|| {
		let bounded_old_maintainer: BoundedVec<u8, ConstU32<1000>> =
			old_maintainer.try_into().unwrap();
		Maintainer::<Test, _>::put(bounded_old_maintainer);
		let mut message = vec![];
		let nonce = 1u32.to_be_bytes();
		message.extend_from_slice(&nonce);
		message.extend_from_slice(&new_maintainer);
		let msg = keccak_256(&message);
		let sig: Signature = pair.sign_prehashed(&msg);

		// set the new maintainer
		assert_ok!(Bridge::set_maintainer(
			RuntimeOrigin::signed(RELAYER_A),
			message.try_into().unwrap(),
			sig.encode().try_into().unwrap()
		));
	})
}

#[test]
fn should_fail_on_invalid_proposal_call() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let r_id = derive_resource_id(1080u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 });
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let call = RuntimeCall::Balances(pallet_balances::Call::transfer { dest: 1, value: 1 });
		let call_encoded = call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);

		// set the new maintainer
		assert_ok!(Bridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));

		// should fail to execute proposal as invalid call is provided
		assert_err!(
			Bridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id,
				prop_data.try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			Error::<Test>::InvalidCall
		);
	})
}
