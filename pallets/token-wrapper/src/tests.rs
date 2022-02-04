use std::{convert::TryInto, ops::Sub};

use crate::mock::*;

use frame_support::{assert_err, assert_ok, error::BadOrigin};

use asset_registry::AssetType;

#[test]
fn should_wrap_token() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));
		println!("{:?}", Tokens::total_issuance(pool_share_id));
		println!("{:?}", TokenWrapper::get_balance(first_token_id, &recipient));
		println!("{:?}", TokenWrapper::get_balance(pool_share_id, &recipient));
		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		// Second argument should be balance minus amount_to_wrap
		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id))
		);

		assert_eq!(TokenWrapper::get_balance(pool_share_id, &recipient), 50000);
	})
}

#[test]
fn should_unwrap_token() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_eq!(TokenWrapper::get_balance(pool_share_id, &recipient), 50000);

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id))
		);

		assert_ok!(TokenWrapper::unwrap(
			Origin::signed(recipient),
			pool_share_id,
			first_token_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), Default::default());

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token.saturating_sub(
				TokenWrapper::get_wrapping_fee(50000, pool_share_id.into()).unwrap()
			)
		);
	})
}

#[test]
fn wrapping_should_fail_if_asset_is_not_in_pool() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_err!(
			TokenWrapper::wrap(
				Origin::signed(recipient),
				first_token_id,
				pool_share_id,
				50000_u128,
				recipient
			),
			crate::Error::<Test>::NotFoundInPool
		);
	})
}

#[test]
fn only_root_should_update_wrapping_fee() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		assert_err!(
			TokenWrapper::set_wrapping_fee(Origin::signed(1), 10, pool_share_id.into()),
			BadOrigin
		);

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 10, pool_share_id.into()));
	})
}

#[test]
fn should_not_unwrap_if_no_liquidity_exists_for_selected_assets() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));

		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_eq!(TokenWrapper::get_balance(pool_share_id, &recipient), 50000);

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id))
		);

		assert_err!(
			TokenWrapper::unwrap(
				Origin::signed(recipient),
				pool_share_id,
				second_token_id,
				50000_u128,
				recipient
			),
			orml_tokens::Error::<Test>::BalanceTooLow
		);
	})
}

#[test]
fn should_unwrap_when_liquidity_exists_for_selected_asset() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));

		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(Currencies::update_balance(
			Origin::root(),
			TokenWrapper::account_id(),
			second_token_id,
			balance
		));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id.into()))
		);

		assert_eq!(TokenWrapper::get_balance(second_token_id, &recipient), Default::default());

		assert_eq!(TokenWrapper::get_balance(pool_share_id, &recipient), 50000);

		assert_ok!(TokenWrapper::unwrap(
			Origin::signed(recipient),
			pool_share_id,
			second_token_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), Default::default());

		assert_eq!(TokenWrapper::get_balance(second_token_id, &recipient), 50000);
	})
}

#[test]
fn should_not_wrap_invalid_amount() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_err!(
			TokenWrapper::wrap(
				Origin::signed(recipient),
				first_token_id,
				pool_share_id,
				0_u128,
				recipient
			),
			crate::Error::<Test>::InvalidAmount
		);
	})
}

#[test]
fn test_two_different_pool_shares() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
			.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
			.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
			.unwrap();

		let third_token_id = AssetRegistry::register_asset(
			b"avax".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
			.unwrap();
		let fourth_token_id = AssetRegistry::register_asset(
			b"cosmos".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
			.unwrap();

		let second_pool_share_id = AssetRegistry::register_asset(
			b"real".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![third_token_id, fourth_token_id]),
			existential_balance.into(),
		)
			.unwrap();

		let recipient: u64 = 1;

		let balance: i128 = 100000;

		let second_recipient: u64 = 1;

		let second_balance: i128 = 100000;

		assert_ok!(Currencies::update_balance(Origin::root(), recipient, first_token_id, balance));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(Currencies::update_balance(Origin::root(), second_recipient, third_token_id, second_balance));
		let initial_balance_third_token = TokenWrapper::get_balance(third_token_id, &second_recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 5, pool_share_id.into()));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 10, second_pool_share_id.into()));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));
		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(second_recipient),
			third_token_id,
			second_pool_share_id,
			50000_u128,
			second_recipient
		));
		assert_eq!(Tokens::total_issuance(second_pool_share_id), 50000);

		// Second argument should be balance minus amount_to_wrap
		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id))
		);

		assert_eq!(
			TokenWrapper::get_balance(third_token_id, &recipient),
			initial_balance_third_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, second_pool_share_id))
		);

	})
}
