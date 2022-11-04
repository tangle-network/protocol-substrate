use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use sp_runtime::AccountId32;
use webb_primitives::webb_proposals::ResourceId;
#[test]
fn set_resource_works() {
	new_test_ext().execute_with(|| {
		// Prepare some balance to pay deposit
		let caller = AccountId32::new([1u8; 32]);
		let resource_id: ResourceId = [1u8; 32].into();
		Balances::make_free_balance_be(&caller, 1000_u32.into());

		assert_ok!(RelayerRegistry::set_resource(
			RuntimeOrigin::signed(caller.clone()),
			resource_id,
			Default::default()
		));

		// ensure the deposit has been deducted
		assert_eq!(Balances::free_balance(&caller), 999_u32.into());

		assert_eq!(
			ResourceOf::<Test>::get(caller, resource_id).unwrap(),
			ResourceRecord { deposit: 1_u32.into(), info: Default::default() }
		)
	});
}

#[test]
fn clear_resource_works() {
	new_test_ext().execute_with(|| {
		// Prepare some balance to pay deposit
		let caller = AccountId32::new([1u8; 32]);
		let resource_id: ResourceId = [1u8; 32].into();
		Balances::make_free_balance_be(&caller, 1000_u32.into());

		assert_ok!(RelayerRegistry::set_resource(
			RuntimeOrigin::signed(caller.clone()),
			resource_id,
			Default::default()
		));

		assert_ok!(RelayerRegistry::clear_resource(
			RuntimeOrigin::signed(caller.clone()),
			resource_id
		));

		// ensure the deposit has been retured
		assert_eq!(Balances::free_balance(&caller), 1000_u32.into());

		assert_eq!(ResourceOf::<Test>::get(caller, resource_id), None)
	});
}
