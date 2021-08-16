use crate::*;
use ark_crypto_primitives::{Error, CRH as CRHTrait};
use ark_ff::{BigInteger, PrimeField};
use arkworks_gadgets::poseidon::{sbox::PoseidonSbox, CircomCRH, PoseidonParameters, Rounds, CRH};
use sp_std::{marker::PhantomData, vec::Vec};

#[derive(Default, Clone, Copy)]
pub struct PoseidonRounds3x5;
#[derive(Default, Clone, Copy)]
pub struct PoseidonRounds5x5;

impl Rounds for PoseidonRounds3x5 {
	const FULL_ROUNDS: usize = 8;
	const PARTIAL_ROUNDS: usize = 57;
	const SBOX: PoseidonSbox = PoseidonSbox::Exponentiation(5);
	const WIDTH: usize = 3;
}

impl Rounds for PoseidonRounds5x5 {
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
		// we use big-endian because it the same for
		// solidity contracts and javascript circom implementations.
		let value = output.into_repr().to_bytes_be();
		Ok(value)
	}
}

pub struct CircomPoseidonHasher<F: PrimeField, P: Rounds>(PhantomData<F>, PhantomData<P>);

impl<F: PrimeField, P: Rounds> InstanceHasher for CircomPoseidonHasher<F, P> {
	fn hash(input: &[u8], param_bytes: &[u8]) -> Result<Vec<u8>, Error> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes)?;
		let output: F = <CircomCRH<F, P> as CRHTrait>::evaluate(&params, input)?;
		// we use big-endian because it the same for
		// solidity contracts and javascript circom implementations.
		let value = output.into_repr().to_bytes_be();
		Ok(value)
	}
}

use ark_bls12_381::Fr as Bls381;
pub type BLS381Poseidon3x5Hasher = ArkworksPoseidonHasher<Bls381, PoseidonRounds3x5>;
pub type BLS381Poseidon5x5Hasher = ArkworksPoseidonHasher<Bls381, PoseidonRounds5x5>;
pub type BLS381CircomPoseidon3x5Hasher = CircomPoseidonHasher<Bls381, PoseidonRounds3x5>;
pub type BLS381CircomPoseidon5x5Hasher = CircomPoseidonHasher<Bls381, PoseidonRounds5x5>;

use ark_bn254::Fr as Bn254;
pub type BN254Poseidon3x5Hasher = ArkworksPoseidonHasher<Bn254, PoseidonRounds3x5>;
pub type BN254Poseidon5x5Hasher = ArkworksPoseidonHasher<Bn254, PoseidonRounds5x5>;
pub type BN254CircomPoseidon3x5Hasher = CircomPoseidonHasher<Bn254, PoseidonRounds3x5>;
pub type BN254CircomPoseidon5x5Hasher = CircomPoseidonHasher<Bn254, PoseidonRounds5x5>;
