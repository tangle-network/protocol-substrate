use std::convert::TryInto;

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

		let fee_recipient: u64 = 10;

		let nonce: u32 = 1;

		assert_ok!(TokenWrapper::set_fee_recipient(
			RuntimeOrigin::root(),
			pool_share_id,
			fee_recipient,
			nonce
		));

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		// increment nonce
		let nonce = nonce + 1;
		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, nonce));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));
		let wrapping_fee = TokenWrapper::get_wrapping_fee(50000_u128, pool_share_id).unwrap();
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
		assert_eq!(TokenWrapper::get_balance(first_token_id, &fee_recipient), wrapping_fee);
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
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
			RuntimeOrigin::signed(recipient),
			pool_share_id,
			first_token_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), Default::default());

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_wrapping_fee(50000, pool_share_id).unwrap())
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_err!(
			TokenWrapper::wrap(
				RuntimeOrigin::signed(recipient),
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
			TokenWrapper::set_wrapping_fee(RuntimeOrigin::signed(1), 10, pool_share_id, 1),
			BadOrigin
		);

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 10, pool_share_id, 1));
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));

		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
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
				RuntimeOrigin::signed(recipient),
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));

		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			TokenWrapper::account_id(),
			second_token_id,
			balance
		));

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &recipient),
			initial_balance_first_token
				.saturating_sub(TokenWrapper::get_amount_to_wrap(50000_u128, pool_share_id))
		);

		assert_eq!(TokenWrapper::get_balance(second_token_id, &recipient), Default::default());

		assert_eq!(TokenWrapper::get_balance(pool_share_id, &recipient), 50000);

		assert_ok!(TokenWrapper::unwrap(
			RuntimeOrigin::signed(recipient),
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_err!(
			TokenWrapper::wrap(
				RuntimeOrigin::signed(recipient),
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

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));
		let initial_balance_first_token = TokenWrapper::get_balance(first_token_id, &recipient);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			second_recipient,
			third_token_id,
			second_balance
		));
		let initial_balance_third_token =
			TokenWrapper::get_balance(third_token_id, &second_recipient);

		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, 1));

		assert_ok!(TokenWrapper::set_wrapping_fee(
			RuntimeOrigin::root(),
			10,
			second_pool_share_id,
			2
		));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));
		assert_eq!(Tokens::total_issuance(pool_share_id), 50000);

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(second_recipient),
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

#[test]
fn should_rescue_all_tokens() {
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

		let fee_recipient: u64 = 10;

		let nonce: u32 = 1;

		assert_ok!(TokenWrapper::set_fee_recipient(
			RuntimeOrigin::root(),
			pool_share_id,
			fee_recipient,
			nonce
		));

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			recipient,
			first_token_id,
			balance
		));

		// increment nonce
		let nonce = nonce + 1;
		assert_ok!(TokenWrapper::set_wrapping_fee(RuntimeOrigin::root(), 5, pool_share_id, nonce));

		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(recipient),
			first_token_id,
			pool_share_id,
			50000_u128,
			recipient
		));
		// Rescue all tokens from fee recipient to provided recipient address.
		let rescue_amount = TokenWrapper::get_balance(first_token_id, &fee_recipient);
		let rescue_tokens_recipient: u64 = 11;
		// increment nonce
		let nonce = nonce + 1;
		assert_ok!(TokenWrapper::rescue_tokens(
			RuntimeOrigin::root(),
			pool_share_id,
			first_token_id,
			rescue_amount,
			rescue_tokens_recipient,
			nonce
		));
		assert_eq!(
			TokenWrapper::get_balance(first_token_id, &rescue_tokens_recipient),
			rescue_amount
		);
	})
}
