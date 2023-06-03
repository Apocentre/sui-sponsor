use serde::Serialize;
use eyre::Result;
use sui_types::{crypto::{PublicKey, Signature, Signer}, base_types::SuiAddress};

use crate::utils::config::KeyPair;

pub struct Wallet {
  keypair: KeyPair,
}

impl Wallet {
  pub fn new(keypair: KeyPair) -> Self {
    Self {
      keypair,
    }
  }

  pub fn public(&self) -> PublicKey {
    self.keypair.public()
  }

  pub fn address(&self) -> SuiAddress {
    (&self.keypair.public()).into()
  }

  pub fn sign<T: Serialize>(&self, msg: &T) -> Result<Signature> {
    Ok(self.keypair.sign(&bincode::serialize(msg)?))
  }
}
