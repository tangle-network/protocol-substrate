use crate::utils::element_encoder;
use ark_bls12_381::Fr as Bls381;
use ark_bn254::Fr as Bn254;
use ark_ff::{BigInteger, PrimeField};
use codec::Encode;
use sp_std::{marker::PhantomData, vec::Vec};

pub trait IntoFieldElement {
	fn into_field<E: Encode>(value: E) -> Vec<u8>;
}

pub struct ArkworksIntoField<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> IntoFieldElement for ArkworksIntoField<F> {
	fn into_field<E: Encode>(value: E) -> Vec<u8> {
		let bytes = value.using_encoded(element_encoder);
		let f = F::from_le_bytes_mod_order(&bytes);
		f.into_repr().to_bytes_le()
	}
}

pub type ArkworksIntoFieldBn254 = ArkworksIntoField<Bn254>;
pub type ArkworksIntoFieldBls381 = ArkworksIntoField<Bls381>;
