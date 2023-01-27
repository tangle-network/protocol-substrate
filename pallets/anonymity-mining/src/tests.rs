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
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// adding AP balance to pallet
		let new_ap_balance = 50000;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
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
		let src_deposit_root = Default::default();
		let src_withdraw_root = Default::default();
		let dest_deposit_root = Default::default();
		let dest_withdraw_root = Default::default();

		let claim_ap_call = AnonymityMining::claim_ap(
			src_resource_id,
			dest_resource_id,
			recipient_one_account_id,
			1000,
			root,
			latest_leaf_index,
			proof_data,
			src_deposit_root,
			src_withdraw_root,
			dest_deposit_root,
			dest_withdraw_root,
		);

		assert_ok!(claim_ap_call);
	})
}

/// Rewards swapping tests

// Test basic set pool weight
#[test]
fn test_basic_set_pool_weight() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));
		assert_eq!(AnonymityMining::get_pool_weight(), 800);

		// Set pool weight to 500
		assert_ok!(AnonymityMining::set_pool_weight(500));
		assert_eq!(AnonymityMining::get_pool_weight(), 500);
	})
}

// Test basic timestamp change
#[test]
fn test_basic_timestamp_change() {
	new_test_ext().execute_with(|| {
		setup_environment();

		let _start_timestamp = AnonymityMining::get_current_timestamp();
		Timestamp::set_timestamp(1);
		let curr_timestamp = AnonymityMining::get_current_timestamp();
		assert_eq!(curr_timestamp, 1);
	})
}

// Test basic get virtual reward balance
#[test]
fn test_basic_get_virtual_reward_balance() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let reward_currency_id = 2;

		let prev_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());

		// Starting virtual balance is at INITIAL_LIQUIDITY
		assert_eq!(prev_virtual_balance, INITIAL_LIQUIDITY);

		// add reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// let now = System::block_timestamp();
		// let timestamp = now + 100;
		// System::set_block_timestamp(timestamp);

		// let start: Moment = (33 * MILLISECS_PER_BLOCK).into();
		// Timestamp::set_timestamp(start);

		//Timestamp::now();

		//Timestamp::set(Timestamp::now() + 100);
		//Timestamp::set(RuntimeOrigin:none(), Timestamp::now() + DURATION + 1);

		let start_timestamp = Timestamp::now();

		// Mid way
		Timestamp::set_timestamp(start_timestamp + DURATION / 2);

		let expected_midway_virtual_balance = INITIAL_LIQUIDITY + LIQUIDITY / 2;
		let midway_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_eq!(
			midway_virtual_balance.saturated_into::<u128>(),
			expected_midway_virtual_balance
		);

		// Final
		Timestamp::set_timestamp(start_timestamp + DURATION + 1);

		let new_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_eq!(new_virtual_balance.saturated_into::<i128>(), new_reward_balance);
	})
}

// Test basic get expected return over time
#[test]
fn test_basic_get_expected_return_varying_timestamp() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 10000
		assert_ok!(AnonymityMining::set_pool_weight(10000));

		let reward_currency_id = 2;

		// add reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		let amount = 10000;

		// Starting timestamp
		// Calc: 10000000(1 - e^-1)
		let starting_expected_return = 6321206;
		let start_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(start_expected_return, starting_expected_return);

		let start_timestamp = Timestamp::now();

		// Midway timestamp
		// Calc: 20000000(1 - e^-1)
		Timestamp::set_timestamp(start_timestamp + DURATION / 2);
		let midway_expected_return = 12642411;

		let mid_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(mid_expected_return.saturated_into::<i128>(), midway_expected_return);

		// Ending timestamp
		// Calc: 30000000(1 - e^-1)
		Timestamp::set_timestamp(start_timestamp + DURATION + 1);
		let ending_expected_return = 18963617;

		let end_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(end_expected_return.saturated_into::<i128>(), ending_expected_return);
	})
}

// Test basic get expected return varying pool weights
#[test]
fn test_basic_get_expected_return_varying_pool_weights() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 10000
		assert_ok!(AnonymityMining::set_pool_weight(10000));

		let reward_currency_id = 2;

		// add reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		let amount = 10000;

		// Calc: 10000000(1 - e^-1)
		let initial_expected_return = 6321206;
		let init_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(init_expected_return, initial_expected_return);

		// Double original pool weight
		assert_ok!(AnonymityMining::set_pool_weight(20000));

		// Calc: 10000000(1 - e^-1/2)
		let another_expected_return = 3934693;
		let other_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(other_expected_return, another_expected_return);

		// Halve original pool weight
		assert_ok!(AnonymityMining::set_pool_weight(5000));

		// Calc: 10000000(1 - e^-2)
		let final_expected_return = 8646647;
		let last_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_eq!(last_expected_return, final_expected_return);
	})
}

// Test basic get expected return varying amounts
#[test]
fn test_basic_get_expected_return_varying_amounts() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 10000
		assert_ok!(AnonymityMining::set_pool_weight(10000));

		let reward_currency_id = 2;

		// add reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		let initial_amount = 10000;

		// Calc: 10000000(1 - e^-1)
		let initial_expected_return = 6321206;
		let init_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), initial_amount);
		assert_eq!(init_expected_return, initial_expected_return);

		// Half amount
		let other_amount = 5000;

		// Calc: 10000000(1 - e^-1/2)
		let another_expected_return = 3934693;
		let other_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), other_amount);
		assert_eq!(other_expected_return, another_expected_return);

		// Double amount
		let final_amount = 20000;

		// Calc: 10000000(1 - e^-2)
		let final_expected_return = 8646647;
		let last_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), final_amount);
		assert_eq!(last_expected_return, final_expected_return);
	})
}

// Test basic swap
#[test]
fn test_basic_swap() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 10000
		assert_ok!(AnonymityMining::set_pool_weight(10000));

		let sender_account_id = account::<AccountId>("", 2, SEED);

		let ap_currency_id = 1;
		let reward_currency_id = 2;

		// check sender AP balance starts at 0
		assert_eq!(Currencies::free_balance(ap_currency_id, &sender_account_id), Zero::zero());

		// adding AP balance to sender
		let new_ap_balance = 10000;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			sender_account_id.clone(),
			ap_currency_id,
			new_ap_balance,
		));

		// check sender AP balance updated
		assert_eq!(
			Currencies::free_balance(ap_currency_id, &sender_account_id),
			new_ap_balance as _
		);

		// check pallet reward balance starts at 0
		assert_eq!(
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id()),
			Zero::zero()
		);

		// adding reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// check pallet reward balances updated
		assert_eq!(
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id()),
			new_reward_balance as _
		);

		// sender and pallet balances before swap
		let sender_ap_balance_before = Currencies::free_balance(ap_currency_id, &sender_account_id);
		let sender_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &sender_account_id);
		let pallet_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		let amount = 10000;

		// Calc: 10000000(1 - e^-1)
		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		assert_eq!(expected_return, 6321206);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_account_id.clone()),
			sender_account_id.clone(),
			amount
		));

		let tokens_sold = AnonymityMining::get_tokens_sold();
		assert_eq!(tokens_sold, 6321206);

		// sender and pallet balances after swap
		let sender_ap_balance_after = Currencies::free_balance(ap_currency_id, &sender_account_id);
		let sender_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &sender_account_id);
		let pallet_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		// check balances update properly
		assert_eq!(sender_ap_balance_after, sender_ap_balance_before - amount);
		assert_eq!(sender_reward_balance_after, sender_reward_balance_before + expected_return);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(pallet_reward_balance_after, pallet_reward_balance_before - expected_return);
	});
}

// Test basic two swaps
#[test]
fn test_basic_two_swaps() {
	new_test_ext().execute_with(|| {
		setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(10000));

		let sender_one_account_id = account::<AccountId>("", 2, SEED);
		let sender_two_account_id = account::<AccountId>("", 3, SEED);

		let ap_currency_id = 1;
		let reward_currency_id = 2;

		// adding AP balance to sender 1
		let new_ap_balance = 50000;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			sender_one_account_id.clone(),
			ap_currency_id,
			new_ap_balance,
		));

		// adding AP balance to sender 2
		let new_ap_balance = 50000;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			sender_two_account_id.clone(),
			ap_currency_id,
			new_ap_balance,
		));

		// adding reward balance to pallet
		let new_reward_balance = INITIAL_TOTAL_REWARDS_BALANCE;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// sender one and pallet balances before swap
		let sender_one_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &sender_one_account_id);
		let sender_one_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &sender_one_account_id);
		let pallet_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		let amount = 10000;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		// Calc: 10000000(1 - e^-1)
		assert_eq!(expected_return, 6321206);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_one_account_id.clone()),
			sender_one_account_id.clone(),
			amount
		));

		// sender and pallet balances after swap
		let sender_one_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &sender_one_account_id);
		let sender_one_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &sender_one_account_id);
		let pallet_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		// check balances update properly
		assert_eq!(sender_one_ap_balance_after, sender_one_ap_balance_before - amount);
		assert_eq!(
			sender_one_reward_balance_after,
			sender_one_reward_balance_before + expected_return
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(pallet_reward_balance_after, pallet_reward_balance_before - expected_return);

		// sender two and pallet balances before swap
		let sender_two_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &sender_two_account_id);
		let sender_two_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &sender_two_account_id);
		let pallet_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		let amount = 10000;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		// Calc: starting = 10000000, prev swap traded for 6321206 tokens
		// So expected return here = (10000000-6321206)(1-e^-1)
		assert_eq!(expected_return, 2325441);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_two_account_id.clone()),
			sender_two_account_id.clone(),
			amount
		));

		// sender and pallet balances after swap
		let sender_two_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &sender_two_account_id);
		let sender_two_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &sender_two_account_id);
		let pallet_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		// check balances update properly
		assert_eq!(sender_two_ap_balance_after, sender_two_ap_balance_before - amount);
		assert_eq!(
			sender_two_reward_balance_after,
			sender_two_reward_balance_before + expected_return
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(pallet_reward_balance_after, pallet_reward_balance_before - expected_return);

		let start_timestamp = Timestamp::now();

		// Half of duration passes
		Timestamp::set_timestamp(start_timestamp + DURATION / 2);

		// Trader 1 trades again

		// sender one and pallet balances before swap
		let sender_one_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &sender_one_account_id);
		let sender_one_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &sender_one_account_id);
		let pallet_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		let amount = 5000;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		// Midway -> now virtual balance base is 20000000
		// Prev tokens sold: 6321206 and 2325441
		// Calc: (20000000-6321206-2325441)(1-e^-(1/2))
		assert_eq!(expected_return, 4467196);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_one_account_id.clone()),
			sender_one_account_id.clone(),
			amount
		));

		// sender and pallet balances after swap
		let sender_one_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &sender_one_account_id);
		let sender_one_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &sender_one_account_id);
		let pallet_ap_balance_after =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_after =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		// check balances update properly
		assert_eq!(sender_one_ap_balance_after, sender_one_ap_balance_before - amount);
		assert_eq!(
			sender_one_reward_balance_after,
			sender_one_reward_balance_before + expected_return
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(pallet_reward_balance_after, pallet_reward_balance_before - expected_return);
	});
}
