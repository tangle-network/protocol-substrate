use crate::*;
use ark_crypto_primitives::{Error, CRH as CRHTrait};
use ark_ff::{BigInteger, PrimeField};
use arkworks_gadgets::poseidon::{sbox::PoseidonSbox, PoseidonParameters, Rounds, CRH};
use sp_std::{marker::PhantomData, vec::Vec};

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
	fn hash(input: &[u8], param_bytes: &[u8]) -> Result<Vec<u8>, Error> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes)?;
		let output: F = <CRH<F, P> as CRHTrait>::evaluate(&params, input)?;
		Ok(output.into_repr().to_bytes_le())
	}
}

use ark_bls12_381::Fr as Bls381;
pub type BLS381Poseidon3Rounds = ArkworksPoseidonHasher<Bls381, PoseidonRounds3>;
pub type BLS381Poseidon5Rounds = ArkworksPoseidonHasher<Bls381, PoseidonRounds5>;
