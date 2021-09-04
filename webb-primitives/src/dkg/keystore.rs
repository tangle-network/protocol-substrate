use std::sync::Arc;
use sp_keystore::{Error, SyncCryptoStore};
pub use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::{Keys};
pub trait MultiPartyECDSAKeyStore: SyncCryptoStore {
	fn generate(&self, index: usize) -> Result<Keys, Error>;
	fn refresh(&self, index: usize) -> Result<Keys, Error> {
		self.generate(index)
	}
}

/// A pointer to a keystore.
pub type MultiPartyCryptoStorePtr = Arc<dyn MultiPartyECDSAKeyStore>;

sp_externalities::decl_extension! {
	/// The keystore extension to register/retrieve from the externalities.
	pub struct KeystoreExt(MultiPartyCryptoStorePtr);
}
