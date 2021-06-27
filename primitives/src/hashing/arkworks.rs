use crate::*;
use sp_std::vec::Vec;
use sp_std::marker::PhantomData;
use ark_ff::{PrimeField, BigInteger};
use arkworks_gadgets::poseidon::{CRH, PoseidonParameters, Rounds, sbox::PoseidonSbox};
use ark_crypto_primitives::{CRH as CRHTrait};

#[derive(Default, Clone)]
pub struct PoseidonRounds3;
#[derive(Default, Clone)]
pub struct PoseidonRounds5;

impl Rounds for PoseidonRounds3 {
	const FULL_ROUNDS: usize = 8;
	const PARTIAL_ROUNDS: usize = 57;
	const SBOX: PoseidonSbox = PoseidonSbox::Exponentiation(5);
	const WIDTH: usize = 3;
}

impl Rounds for PoseidonRounds5 {
	const FULL_ROUNDS: usize = 8;
	const PARTIAL_ROUNDS: usize = 60;
	const SBOX: PoseidonSbox = PoseidonSbox::Exponentiation(5);
	const WIDTH: usize = 5;
}

pub struct ArkworksPoseidonHasher<F: PrimeField, P: Rounds>(PhantomData<F>, PhantomData<P>);

impl<F: PrimeField, P: Rounds> InstanceHasher for ArkworksPoseidonHasher<F, P> {
	fn hash(input: &[u8], param_bytes: &[u8]) -> Vec<u8> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes).unwrap();
		let output: F = <CRH::<F, P> as CRHTrait>::evaluate(&params, input).unwrap_or(Default::default());
		output.into_repr().to_bytes_le()
	}
}

use ark_bls12_381::{Fr as Bls381};
pub type BLS381Poseidon3Rounds = ArkworksPoseidonHasher<Bls381, PoseidonRounds3>;
pub type BLS381Poseidon5Rounds = ArkworksPoseidonHasher<Bls381, PoseidonRounds5>;