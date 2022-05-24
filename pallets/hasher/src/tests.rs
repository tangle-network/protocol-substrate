use super::*;
use crate::mock::*;
use ark_ff::prelude::*;
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use sp_core::bytes;

type Fr = ark_bn254::Fr;

#[test]
fn should_fail_with_params_not_initialized() {
	new_test_ext().execute_with(|| {
		assert_err!(
			<DefaultPalletHasher as HasherModule>::hash(&[1u8; 32]),
			Error::<Test>::ParametersNotInitialized
		);
	});
}

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<Fr>(curve, 5, 3);
		let res = DefaultPalletHasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
	});
}

#[test]
fn should_output_correct_hash() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<Fr>(curve, 5, 3);
		let res = DefaultPalletHasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
		let left = Fr::one().into_repr().to_bytes_le(); // one
		let right = Fr::one().double().into_repr().to_bytes_le(); // two
		let hash = DefaultPalletHasher::hash_two(&left, &right).unwrap();
		let f = Fr::from_le_bytes_mod_order(&hash).into_repr().to_bytes_be();
		assert_eq!(
			f,
			bytes::from_hex("0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a")
				.unwrap()
		);
	});
}

#[test]
fn should_build_the_same_merkle_tree_solidity() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<Fr>(curve, 5, 3);
		let res = DefaultPalletHasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
		let solidity_merkle_tree_hashes = vec![
			hex!("2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c"),
			hex!("13e37f2d6cb86c78ccc1788607c2b199788c6bb0a615a21f2e7a8e88384222f8"),
			hex!("217126fa352c326896e8c2803eec8fd63ad50cf65edfef27a41a9e32dc622765"),
			hex!("0e28a61a9b3e91007d5a9e3ada18e1b24d6d230c618388ee5df34cacd7397eee"),
			hex!("27953447a6979839536badc5425ed15fadb0e292e9bc36f92f0aa5cfa5013587"),
			hex!("194191edbfb91d10f6a7afd315f33095410c7801c47175c2df6dc2cce0e3affc"),
			hex!("1733dece17d71190516dbaf1927936fa643dc7079fc0cc731de9d6845a47741f"),
			hex!("267855a7dc75db39d81d17f95d0a7aa572bf5ae19f4db0e84221d2b2ef999219"),
			hex!("1184e11836b4c36ad8238a340ecc0985eeba665327e33e9b0e364127c27620d"),
			hex!("0702ab83a135d7f55350ab1bfaa90babd8fc1d2b3e6a7215381a7b2213d6c5ce"),
			hex!("2eecc0de814cfd8c57ce882babb2e30d1da56621aef7a47f3291cffeaec26ad7"),
			hex!("280bc02145c155d5833585b6c7b08501055157dd30ce05319621dc462d33b47"),
			hex!("045132221d1fa0a7f4aed8acd2cbec1e2189b7732ccb2ec272b9c60f0d5afc5b"),
			hex!("27f427ccbf58a44b1270abbe4eda6ba53bd6ac4d88cf1e00a13c4371ce71d366"),
			hex!("1617eaae5064f26e8f8a6493ae92bfded7fde71b65df1ca6d5dcec0df70b2cef"),
			hex!("20c6b400d0ea1b15435703c31c31ee63ad7ba5c8da66cec2796feacea575abca"),
			hex!("09589ddb438723f53a8e57bdada7c5f8ed67e8fece3889a73618732965645eec"),
			hex!("0064b6a738a5ff537db7b220f3394f0ecbd35bfd355c5425dc1166bf3236079b"),
			hex!("095de56281b1d5055e897c3574ff790d5ee81dbc5df784ad2d67795e557c9e9f"),
			hex!("11cf2e2887aa21963a6ec14289183efe4d4c60f14ecd3d6fe0beebdf855a9b63"),
			hex!("2b0f6fc0179fa65b6f73627c0e1e84c7374d2eaec44c9a48f2571393ea77bcbb"),
			hex!("16fdb637c2abf9c0f988dbf2fd64258c46fb6a273d537b2cf1603ea460b13279"),
			hex!("21bbd7e944f6124dad4c376df9cc12e7ca66e47dff703ff7cedb1a454edcf0ff"),
			hex!("2784f8220b1c963e468f590f137baaa1625b3b92a27ad9b6e84eb0d3454d9962"),
			hex!("16ace1a65b7534142f8cc1aad810b3d6a7a74ca905d9c275cb98ba57e509fc10"),
			hex!("2328068c6a8c24265124debd8fe10d3f29f0665ea725a65e3638f6192a96a013"),
			hex!("2ddb991be1f028022411b4c4d2c22043e5e751c120736f00adf54acab1c9ac14"),
			hex!("0113798410eaeb95056a464f7521eb58377c155f2fe518a5594d38cc209cc0"),
			hex!("202d1ae61526f0d0d01ef80fb5d4055a7af45721024c2c24cffd6a3798f54d50"),
			hex!("23ab323453748129f2765f79615022f5bebd6f496a796300aab049a60b0f187"),
			hex!("1f15585f8947e378bcf8bd918716799da909acdb944c57150b1eb4565fda8aa0"),
			hex!("1eb064b21055ac6a350cf41eb30e4ce2cb19680217df3a243617c2838185ad06"),
		];
		let default_zero = [
			108, 175, 153, 72, 237, 133, 150, 36, 226, 65, 231, 118, 15, 52, 27, 130, 180, 93, 161,
			235, 182, 53, 58, 52, 243, 171, 172, 211, 96, 76, 229, 47,
		]
		.to_vec();

		let other_zero = [
			47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52,
			15, 118, 231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
		]
		.to_vec();

		let mut hashes = vec![];
		let mut temp_hash_bytes = default_zero;
		hashes.push(temp_hash_bytes.clone());
		for i in 0..32 {
			temp_hash_bytes =
				DefaultPalletHasher::hash_two(&temp_hash_bytes, &temp_hash_bytes).unwrap();
			hashes.push(temp_hash_bytes.clone());
		}

		for i in 0..31 {
			println!("{:?}\n{:?}\n", hashes[i], solidity_merkle_tree_hashes[i],);
		}
	});
}
