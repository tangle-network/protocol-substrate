use std::convert::TryInto;

use crate::mock::*;

use frame_support::{assert_ok, traits::Currency};

use asset_registry::AssetType;

#[test]
fn should_wrap_token() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(b"shib".to_vec().try_into().unwrap(), AssetType::Token, existential_balance.into()).unwrap();
		let second_token_id = AssetRegistry::register_asset(b"doge".to_vec().try_into().unwrap(), AssetType::Token, existential_balance.into()).unwrap();

		let pool_share_id = AssetRegistry::register_asset(b"meme".to_vec().try_into().unwrap(), AssetType::PoolShare(vec![second_token_id,first_token_id]), existential_balance.into()).unwrap();

		let recipient: u64 = 1;
		let currency_id: u32 = 1;
		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance.into()));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5));

		assert_ok!(TokenWrapper::wrap(Origin::signed(recipient), first_token_id, pool_share_id, 50000 as u128, recipient));
		
		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);
	})
}

#[test]
fn should_unwrap_token() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(b"shib".to_vec().try_into().unwrap(), AssetType::Token, existential_balance.into()).unwrap();
		let second_token_id = AssetRegistry::register_asset(b"doge".to_vec().try_into().unwrap(), AssetType::Token, existential_balance.into()).unwrap();

		let pool_share_id = AssetRegistry::register_asset(b"meme".to_vec().try_into().unwrap(), AssetType::PoolShare(vec![second_token_id,first_token_id]), existential_balance.into()).unwrap();

		let recipient: u64 = 1;
		let currency_id: u32 = 1;
		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance.into()));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5));

		assert_ok!(TokenWrapper::wrap(Origin::signed(recipient), first_token_id, pool_share_id, 50000 as u128, recipient));
		
		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_ok!(TokenWrapper::unwrap(Origin::signed(recipient),pool_share_id, first_token_id, 50000 as u128, recipient ));

		assert_eq!(Tokens::total_issuance(pool_share_id), Default::default());
	})
}

#[test]
fn wrapping_should_fail_if_asset_is_not_in_pool() {
	assert_eq!(true, false)
}

#[test]
fn only_root_should_update_wrapping_fee() {
	assert_eq!(true, false)
}

#[test]
fn should_not_unwrap_if_no_liquidity_exists_for_selected_assets() {
	assert_eq!(true, false)
}

#[test]
fn should_unwrap_when_liquidity_exists_for_selected_asset() {
	assert_eq!(true, false)
}
