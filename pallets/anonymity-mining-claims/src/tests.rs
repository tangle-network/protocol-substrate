use super::*;
use crate::mock::*;
use std::default::Default;

use frame_benchmarking::account;
use frame_support::assert_ok;

use sp_runtime::traits::Zero;

use webb_primitives::{
	types::vanchor::ProofData,
	webb_proposals::{
		FunctionSignature, ResourceId, SubstrateTargetSystem, TargetSystem, TypedChainId,
	},
};

use arkworks_setups::{common::setup_params, Curve};

use pallet_vanchor_handler::AnchorList;

const SEED: u32 = 0;
const START_TIMESTAMP: u64 = 0;
const INITIAL_LIQUIDITY: u128 = 10000000;
const LIQUIDITY: u128 = 20000000;
const INITIAL_TOTAL_REWARDS_BALANCE: i128 = 30000000;
const DURATION: u64 = 31536000;

const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {});
}

fn setup_environment() {
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
	] {
		assert_ok!(Balances::set_balance(RuntimeOrigin::root(), account_id, 100_000_000, 0));
	}
}

// helper function to create anchor using Anchor pallet call
fn mock_vanchor_creation_using_pallet_call(resource_id: &ResourceId) {
	assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
	assert_ok!(VAnchor::create(RuntimeOrigin::root(), TEST_MAX_EDGES, TEST_TREE_DEPTH, 0));
	AnchorList::<Test>::insert(resource_id, 0);
	assert!(<pallet_mt::Trees<Test>>::contains_key(0));
	assert_eq!(TEST_MAX_EDGES, <pallet_linkable_tree::MaxEdges<Test>>::get(0));
}

/// AP claim tests

// Test claim_ap
#[test]
fn test_claim_ap() {
	new_test_ext().execute_with(|| {
		setup_environment();

		let recipient_one_account_id = account::<AccountId>("", 2, SEED);
		let sender_two_account_id = account::<AccountId>("", 3, SEED);

		let src_id = TypedChainId::Substrate(1);
		let target_id = TypedChainId::Substrate(5);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
		let r_id: ResourceId = ResourceId::new(target_system, target_id);

		let root = Element::from_bytes(&[1; 32]);
		let latest_leaf_index = 5;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let dest_target_system = target_system;
		let dest_resource_id = ResourceId::new(dest_target_system, target_id);

		// print out r_id
		println!("r_id: {:?}", r_id);

		let tree_id = 5;

		// token setup
		let ap_currency_id = 1;
		let reward_currency_id = 2;

		// add reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMiningClaims::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// adding AP balance to pallet
		let new_ap_balance = 50000;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMiningClaims::account_id(),
			ap_currency_id,
			new_ap_balance,
		));

		// param setup
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let _ = HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params.to_bytes().try_into().unwrap(),
		);

		SignatureBridge::whitelist_chain(RuntimeOrigin::root(), src_id.chain_id());
		SignatureBridge::set_resource(RuntimeOrigin::root(), r_id);
		SignatureBridge::resource_exists(r_id);

		mock_vanchor_creation_using_pallet_call(&r_id);

		// mock proof data
		let proof_data = ProofData {
			proof: vec![],
			public_amount: Default::default(),
			roots: vec![],
			input_nullifiers: vec![],
			output_commitments: vec![],
			ext_data_hash: Default::default(),
		};

		// mock roots
		let deposit_root = Default::default();
		let withdraw_root = Default::default();

		let claim_ap_call = AnonymityMiningClaims::claim_ap(
			src_resource_id,
			dest_resource_id,
			recipient_one_account_id,
			1000,
			root,
			latest_leaf_index,
			proof_data,
			deposit_root,
			withdraw_root,
		);

		assert_ok!(claim_ap_call);
	})
}
