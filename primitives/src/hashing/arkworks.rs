use crate::*;
use sp_std::marker::PhantomData;
use ark_ff::{PrimeField, BigInteger};
use arkworks_gadgets::poseidon::{CRH, PoseidonParameters, Rounds};
use ark_crypto_primitives::{CRH as CRHTrait};

pub struct ArkworksPoseidonHasher<F: PrimeField, P: Rounds>(PhantomData<F>, PhantomData<P>);

impl<F: PrimeField, P: Rounds> InstanceHasher for ArkworksPoseidonHasher<F, P> {
	fn hash(input: &[u8], param_bytes: &[u8]) -> Vec<u8> {
		let params = PoseidonParameters::<F>::from_bytes(param_bytes).unwrap();
		let output: F = <CRH::<F, P> as CRHTrait>::evaluate(&params, input).unwrap_or(Default::default());
		output.into_repr().to_bytes_le()
	}
}