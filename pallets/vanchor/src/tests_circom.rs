use crate::{
	mock::*,
	test_utils::{deconstruct_public_inputs_el, setup_utxos, setup_zk_circuit},
	tests::*,
	Error, Instance1, MaxDepositAmount, MinWithdrawAmount,
};

use ark_bn254::Bn254;
use ark_circom::{read_zkey, CircomConfig};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::ProvingKey;
use arkworks_setups::{common::setup_params, utxo::Utxo, Curve};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use orml_traits::MultiCurrency;
use pallet_asset_registry::AssetType;
use pallet_linkable_tree::LinkableTreeConfigration;
use sp_core::hashing::keccak_256;
use std::{convert::TryInto, fs::File};
use webb_primitives::{
	linkable_tree::LinkableTreeInspector,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::compute_chain_id_type,
	verifying::VerifyingKey,
	AccountId,
};

type Bn254Fr = ark_bn254::Fr;

fn setup_environment_with_circom() -> (Vec<u8>, Vec<u8>, ProvingKey<Bn254>, ProvingKey<Bn254>) {
	let curve = Curve::Bn254;
	let params3 = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	// 1. Setup The Hasher Pallet.
	assert_ok!(Hasher1::force_set_parameters(RuntimeOrigin::root(), params3.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree1 as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	// Load the WASM and R1CS for witness and proof generation
	let cfg_2_2 = CircomConfig::<Bn254>::new(
		"../../../solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.wasm",
		"../../../solidity-fixtures/vanchor_2/2/poseidon_vanchor_2_2.r1cs",
	)
	.unwrap();

	let cfg_16_2 = CircomConfig::<Bn254>::new(
		"../../../solidity-fixtures/vanchor_16/2/poseidon_vanchor_2_2.wasm",
		"../../../solidity-fixtures/vanchor_16/2/poseidon_vanchor_2_2.r1cs",
	)
	.unwrap();

	let path_2_2 = "../../../solidity-fixtures/vanchor_2/2/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let (params_2_2, _matrices) = read_zkey(&mut file_2_2).unwrap();

	let path_16_2 = "../../../solidity-fixtures/vanchor_16/2/circuit_final.zkey";
	let mut file_16_2 = File::open(path_16_2).unwrap();
	let (params_16_2, _matrices) = read_zkey(&mut file_16_2).unwrap();

	let vk_2_2: VerifyingKey = params_2_2.vk.clone().into();
	let vk_2_2_bytes = vk_2_2.to_bytes();

	let vk_2_16: VerifyingKey = params_16_2.vk.clone().into();
	let vk_2_16_bytes = vk_2_16.to_bytes();

	assert_ok!(VAnchorVerifier1::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 2),
		vk_2_2_bytes.clone()
	));
	assert_ok!(VAnchorVerifier1::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 16),
		vk_2_16_bytes.clone()
	));

	let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	let relayer = account::<AccountId>("", RELAYER_ACCOUNT_ID, SEED);
	let big_transactor = account::<AccountId>("", BIG_TRANSACTOR_ACCOUNT_ID, SEED);
	let bigger_transactor = account::<AccountId>("", BIGGER_TRANSACTOR_ACCOUNT_ID, SEED);

	// Set balances
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), transactor, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), relayer, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		big_transactor,
		BIG_DEFAULT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		bigger_transactor,
		BIGGER_DEFAULT_BALANCE,
		0
	));

	// set configurable storage
	assert_ok!(VAnchor1::set_max_deposit_amount(RuntimeOrigin::root(), 10, 1));
	assert_ok!(VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 3, 2));

	// finally return the provingkey bytes
	(vk_2_2_bytes, vk_2_16_bytes, params_2_2, params_16_2)
}

#[test]
fn should_complete_2x2_transaction_with_withdraw() {
	new_test_ext().execute_with(|| {});
}
