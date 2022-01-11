use codec::{Encode, Decode};

pub mod ecdsa;

pub trait SigningSystem<Public, Signature>
where
    Public: PartialEq + Encode,
    Signature: Encode
{
    type Error;

    fn verify(
        key: &Public,
        msg: &[u8],
        sig: &Signature,
    ) -> Result<bool, Self::Error> {
        let public_key = Self::recover_pub_key(msg, sig)?;
        Ok(public_key == *key)
    }

    fn recover_pub_key(
        msg: &[u8],
        sig: &Signature,
    ) -> Result<Public, Self::Error>;
}