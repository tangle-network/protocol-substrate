use super::*;
use crate::mock::*;
use ark_ff::prelude::*;
use arkworks_setups::{common::setup_params, Curve};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use sp_core::bytes;
use sp_runtime::traits::{One, Zero};

const SEED: u32 = 0;
const INITIAL_LIQUIDITY: u128 = 10000000;
const DURATION: u64 = 31536000;

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {});
}

fn setup_environment() {
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
	] {
		assert_ok!(Balances::set_balance(RuntimeOrigin::root(), account_id, 100_000_000, 0));
	}
}

// Test basic set pool weight
#[test]
fn test_basic_set_pool_weight() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

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
		let _ = setup_environment();

		let startTimestamp = AnonymityMining::get_current_timestamp().unwrap();

		Timestamp::set(RuntimeOrigin::root(), Timestamp::now() + DURATION + 1);

		// TODO: why is this not working
		let newTimestamp = AnonymityMining::get_current_timestamp().unwrap();
		assert_eq!(newTimestamp, 0);
	})
}

// Test basic get virtual reward balance - after elapsed time
#[test]
fn test_basic_get_virtual_reward_balance() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let reward_currency_id = 2;

		let prev_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_ok!(prev_virtual_balance);
		assert_eq!(prev_virtual_balance.unwrap(), 0);

		// add reward balance to pallet
		let new_reward_balance = 100;
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
		Timestamp::set(RuntimeOrigin::root(), Timestamp::now() + DURATION + 1);

		let new_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_ok!(new_virtual_balance);
		assert_eq!(new_virtual_balance.unwrap().saturated_into::<i128>(), new_reward_balance);
	})
}

// Test basic get virtual reward balance after
#[test]
fn test_basic_get_virtual_reward_balance_modified_timestamp() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let reward_currency_id = 2;

		let prev_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_ok!(prev_virtual_balance);
		assert_eq!(prev_virtual_balance.unwrap(), 0);

		// add reward balance to pallet
		let new_reward_balance = 100;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		// TODO: factor in
		Timestamp::set(RuntimeOrigin::root(), Timestamp::now() + 31536000);

		let new_virtual_balance =
			AnonymityMining::get_virtual_balance(&AnonymityMining::account_id());
		assert_ok!(new_virtual_balance);
		assert_eq!(new_virtual_balance.unwrap().saturated_into::<i128>(), new_reward_balance);
	})
}

// Test basic get expected return
#[test]
fn test_basic_get_expected_return() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let reward_currency_id = 2;

		let amount = 100;
		let prev_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_ok!(prev_expected_return);
		assert_eq!(prev_expected_return.unwrap(), 0);

		// add reward balance to pallet
		let new_reward_balance = 100;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			AnonymityMining::account_id(),
			reward_currency_id,
			new_reward_balance,
		));

		let expected_return_num = 12;

		let new_expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);
		assert_ok!(new_expected_return);
		assert_eq!(new_expected_return.unwrap().saturated_into::<i128>(), expected_return_num);
	})
}

// Test basic swap
#[test]
fn test_basic_swap() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let sender_account_id = account::<AccountId>("", 2, SEED);

		let ap_currency_id = 1;
		let reward_currency_id = 2;

		// check sender AP balance starts at 0
		assert_eq!(Currencies::free_balance(ap_currency_id, &sender_account_id), Zero::zero());

		// adding AP balance to sender
		let new_ap_balance = 100;
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
		let new_reward_balance = 100;
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

		let amount = 100;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_account_id.clone()),
			sender_account_id,
			amount
		));

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
		assert_eq!(
			sender_reward_balance_after,
			sender_reward_balance_before + expected_return.unwrap()
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(
			pallet_reward_balance_after,
			pallet_reward_balance_before - expected_return.unwrap()
		);
	});
}

// Test basic two swaps
#[test]
fn test_basic_two_swaps() {
	new_test_ext().execute_with(|| {
		let _ = setup_environment();

		// Set pool weight to 800
		assert_ok!(AnonymityMining::set_pool_weight(800));

		let sender_one_account_id = account::<AccountId>("", 2, SEED);
		let sender_two_account_id = account::<AccountId>("", 3, SEED);

		let ap_currency_id = 1;
		let reward_currency_id = 2;

		// adding AP balance to sender 1
		let new_ap_balance = 100;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			sender_one_account_id.clone(),
			ap_currency_id,
			new_ap_balance,
		));

		// adding AP balance to sender 2
		let new_ap_balance = 100;
		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			sender_two_account_id.clone(),
			ap_currency_id,
			new_ap_balance,
		));

		// adding reward balance to pallet
		let new_reward_balance = 100;
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

		let amount = 100;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		assert_eq!(expected_return.unwrap(), 12);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_one_account_id.clone()),
			sender_one_account_id,
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
			sender_one_reward_balance_before + expected_return.unwrap()
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(
			pallet_reward_balance_after,
			pallet_reward_balance_before - expected_return.unwrap()
		);

		// sender two and pallet balances before swap
		let sender_two_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &sender_two_account_id);
		let sender_two_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &sender_two_account_id);
		let pallet_ap_balance_before =
			Currencies::free_balance(ap_currency_id, &AnonymityMining::account_id());
		let pallet_reward_balance_before =
			Currencies::free_balance(reward_currency_id, &AnonymityMining::account_id());

		let amount = 100;

		let expected_return =
			AnonymityMining::get_expected_return(&AnonymityMining::account_id(), amount);

		assert_eq!(expected_return.unwrap(), 10);

		// conduct swap
		assert_ok!(AnonymityMining::swap(
			RuntimeOrigin::signed(sender_two_account_id.clone()),
			sender_two_account_id,
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
			sender_two_reward_balance_before + expected_return.unwrap()
		);
		assert_eq!(pallet_ap_balance_after, pallet_ap_balance_before + amount);
		assert_eq!(
			pallet_reward_balance_after,
			pallet_reward_balance_before - expected_return.unwrap()
		);
	});
}
