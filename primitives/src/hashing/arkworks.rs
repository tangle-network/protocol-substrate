use crate::*;
use ark_crypto_primitives::{Error, CRH as CRHTrait};
use ark_ff::{BigInteger, PrimeField};
use arkworks_gadgets::poseidon::CRH;
use arkworks_utils::poseidon::PoseidonParameters;
use sp_std::{marker::PhantomData, vec::Vec};

pub struct ArkworksPoseidonHasher<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> InstanceHasher for ArkworksPoseidonHasher<F> {
	fn hash(input: &[u8], param_bytes: &[u8]) -> Result<Vec<u8>, Error> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes)?;
		let output: F = <CRH<F> as CRHTrait>::evaluate(&params, input)?;
		let value = output.into_repr().to_bytes_le();
		Ok(value)
	}
}

pub struct CircomPoseidonHasher<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> InstanceHasher for CircomPoseidonHasher<F> {
	fn hash(input: &[u8], param_bytes: &[u8]) -> Result<Vec<u8>, Error> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes)?;
		let output: F = <CRH<F> as CRHTrait>::evaluate(&params, input)?;
		let value = output.into_repr().to_bytes_le();
		Ok(value)
	}
}

use ark_bls12_381::Fr as Bls381;
pub type BLS381Poseidon3x5Hasher = ArkworksPoseidonHasher<Bls381>;
pub type BLS381Poseidon5x5Hasher = ArkworksPoseidonHasher<Bls381>;
pub type BLS381CircomPoseidon3x5Hasher = CircomPoseidonHasher<Bls381>;
pub type BLS381CircomPoseidon5x5Hasher = CircomPoseidonHasher<Bls381>;

use ark_bn254::Fr as Bn254;
pub type BN254Poseidon3x5Hasher = ArkworksPoseidonHasher<Bn254>;
pub type BN254Poseidon5x5Hasher = ArkworksPoseidonHasher<Bn254>;
pub type BN254CircomPoseidon3x5Hasher = CircomPoseidonHasher<Bn254>;
pub type BN254CircomPoseidon5x5Hasher = CircomPoseidonHasher<Bn254>;
