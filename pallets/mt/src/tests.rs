use ark_ff::{BigInteger, PrimeField};
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use sp_runtime::ModuleError;
use sp_std::vec;

use super::*;
use crate::mock::*;

#[test]
fn shout_create_an_empty_tree() {
	new_test_ext().execute_with(|| {
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		assert_ok!(MerkleTree::create(RuntimeOrigin::signed(1), 32));
	});
}

#[test]
fn should_fail_in_case_of_larger_depth() {
	new_test_ext().execute_with(|| {
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		let max_depth = <Test as Config>::MaxTreeDepth::get();
		assert_err!(
			MerkleTree::create(RuntimeOrigin::signed(1), max_depth + 1),
			DispatchError::Module(ModuleError {
				index: 3,
				error: [1, 0, 0, 0], // InvalidTreeDepth,
				message: None,
			})
		);
	});
}

#[test]
fn should_fail_in_case_when_max_default_hashes_is_exceeded() {
	new_test_ext().execute_with(|| {
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		let max_default_hashes = <Test as Config>::MaxTreeDepth::get();
		assert_err!(
			MerkleTree::force_set_default_hashes(
				RuntimeOrigin::root(),
				vec![
					<Test as Config>::DefaultZeroElement::get();
					(max_default_hashes + 1) as usize
				]
				.try_into()
				.unwrap()
			),
			crate::Error::<Test, _>::ExceedsMaxDefaultHashes
		);
	});
}

#[test]
fn should_successfully_set_default_hashes_to_match_solidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		assert_ok!(MerkleTree::create(RuntimeOrigin::signed(1), 32));
		let default_hashes: Vec<Element> = MerkleTree::default_hashes().into_inner();
		let solidity_merkle_tree_hashes: Vec<Element> = vec![
			Element::from_bytes(&hex!(
				"2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c"
			)),
			Element::from_bytes(&hex!(
				"13e37f2d6cb86c78ccc1788607c2b199788c6bb0a615a21f2e7a8e88384222f8"
			)),
			Element::from_bytes(&hex!(
				"217126fa352c326896e8c2803eec8fd63ad50cf65edfef27a41a9e32dc622765"
			)),
			Element::from_bytes(&hex!(
				"0e28a61a9b3e91007d5a9e3ada18e1b24d6d230c618388ee5df34cacd7397eee"
			)),
			Element::from_bytes(&hex!(
				"27953447a6979839536badc5425ed15fadb0e292e9bc36f92f0aa5cfa5013587"
			)),
			Element::from_bytes(&hex!(
				"194191edbfb91d10f6a7afd315f33095410c7801c47175c2df6dc2cce0e3affc"
			)),
			Element::from_bytes(&hex!(
				"1733dece17d71190516dbaf1927936fa643dc7079fc0cc731de9d6845a47741f"
			)),
			Element::from_bytes(&hex!(
				"267855a7dc75db39d81d17f95d0a7aa572bf5ae19f4db0e84221d2b2ef999219"
			)),
			Element::from_bytes(&hex!(
				"1184e11836b4c36ad8238a340ecc0985eeba665327e33e9b0e3641027c27620d"
			)),
			Element::from_bytes(&hex!(
				"0702ab83a135d7f55350ab1bfaa90babd8fc1d2b3e6a7215381a7b2213d6c5ce"
			)),
			Element::from_bytes(&hex!(
				"2eecc0de814cfd8c57ce882babb2e30d1da56621aef7a47f3291cffeaec26ad7"
			)),
			Element::from_bytes(&hex!(
				"280bc02145c155d5833585b6c7b08501055157dd30ce005319621dc462d33b47"
			)),
			Element::from_bytes(&hex!(
				"045132221d1fa0a7f4aed8acd2cbec1e2189b7732ccb2ec272b9c60f0d5afc5b"
			)),
			Element::from_bytes(&hex!(
				"27f427ccbf58a44b1270abbe4eda6ba53bd6ac4d88cf1e00a13c4371ce71d366"
			)),
			Element::from_bytes(&hex!(
				"1617eaae5064f26e8f8a6493ae92bfded7fde71b65df1ca6d5dcec0df70b2cef"
			)),
			Element::from_bytes(&hex!(
				"20c6b400d0ea1b15435703c31c31ee63ad7ba5c8da66cec2796feacea575abca"
			)),
			Element::from_bytes(&hex!(
				"09589ddb438723f53a8e57bdada7c5f8ed67e8fece3889a73618732965645eec"
			)),
			Element::from_bytes(&hex!(
				"0064b6a738a5ff537db7b220f3394f0ecbd35bfd355c5425dc1166bf3236079b"
			)),
			Element::from_bytes(&hex!(
				"095de56281b1d5055e897c3574ff790d5ee81dbc5df784ad2d67795e557c9e9f"
			)),
			Element::from_bytes(&hex!(
				"11cf2e2887aa21963a6ec14289183efe4d4c60f14ecd3d6fe0beebdf855a9b63"
			)),
			Element::from_bytes(&hex!(
				"2b0f6fc0179fa65b6f73627c0e1e84c7374d2eaec44c9a48f2571393ea77bcbb"
			)),
			Element::from_bytes(&hex!(
				"16fdb637c2abf9c0f988dbf2fd64258c46fb6a273d537b2cf1603ea460b13279"
			)),
			Element::from_bytes(&hex!(
				"21bbd7e944f6124dad4c376df9cc12e7ca66e47dff703ff7cedb1a454edcf0ff"
			)),
			Element::from_bytes(&hex!(
				"2784f8220b1c963e468f590f137baaa1625b3b92a27ad9b6e84eb0d3454d9962"
			)),
			Element::from_bytes(&hex!(
				"16ace1a65b7534142f8cc1aad810b3d6a7a74ca905d9c275cb98ba57e509fc10"
			)),
			Element::from_bytes(&hex!(
				"2328068c6a8c24265124debd8fe10d3f29f0665ea725a65e3638f6192a96a013"
			)),
			Element::from_bytes(&hex!(
				"2ddb991be1f028022411b4c4d2c22043e5e751c120736f00adf54acab1c9ac14"
			)),
			Element::from_bytes(&hex!(
				"0113798410eaeb95056a464f70521eb58377c0155f2fe518a5594d38cc209cc0"
			)),
			Element::from_bytes(&hex!(
				"202d1ae61526f0d0d01ef80fb5d4055a7af45721024c2c24cffd6a3798f54d50"
			)),
			Element::from_bytes(&hex!(
				"23ab323453748129f2765f79615022f5bebd6f4096a796300aab049a60b0f187"
			)),
			Element::from_bytes(&hex!(
				"1f15585f8947e378bcf8bd918716799da909acdb944c57150b1eb4565fda8aa0"
			)),
			Element::from_bytes(&hex!(
				"1eb064b21055ac6a350cf41eb30e4ce2cb19680217df3a243617c2838185ad06"
			)),
		];
		for i in 0..solidity_merkle_tree_hashes.len() {
			println!("{i:?}");
			println!(
				"{:?}\n{:?}\n",
				default_hashes[i].to_bytes(),
				solidity_merkle_tree_hashes[i].to_bytes()
			);

			if i > 0 {
				assert_eq!(
					ark_bn254::Fr::from_be_bytes_mod_order(default_hashes[i].to_bytes()),
					ark_bn254::Fr::from_be_bytes_mod_order(
						solidity_merkle_tree_hashes[i].to_bytes()
					)
				);
			}
		}
	});
}

#[test]
fn should_be_able_to_insert_leaves() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(MerkleTree::create(RuntimeOrigin::signed(1), depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&ark_bn254::Fr::from(1).into_repr().to_bytes_be());
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(MerkleTree::insert(RuntimeOrigin::signed(1), tree_id, leaf));
		});
	});
}

#[test]
fn should_fail_if_the_tree_is_full() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(RuntimeOrigin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(MerkleTree::create(RuntimeOrigin::signed(1), depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&[1u8; 32]);
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(MerkleTree::insert(RuntimeOrigin::signed(1), tree_id, leaf));
		});
		assert_err!(
			MerkleTree::insert(RuntimeOrigin::signed(1), tree_id, leaf),
			DispatchError::Module(ModuleError {
				index: 3,
				error: [3, 0, 0, 0], // ExceedsMaxLeaves
				message: None,
			})
		);
	});
}

#[test]
fn genesis_config_works() {
	ExtBuilder::default().with_crate_gen_config().execute_with(|| {
		assert!(!MerkleTree::is_default_hashes_empty());
	})
}

#[ignore = "used only for debugging"]
#[test]
pub fn shout_print_zero_element() {
	use ark_bn254::Fr;
	let f = ark_ff::field_new!(
		Fr,
		"21663839004416932945382355908790599225266501822907911457504978515578255421292"
	);
	let f_bytes = f.into_repr().to_bytes_be();
	dbg!(Element::from_bytes(&f_bytes));
}
