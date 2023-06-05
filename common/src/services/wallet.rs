use serde::Serialize;
use eyre::Result;
use shared_crypto::intent::{IntentMessage, Intent};
use sui_types::{crypto::{PublicKey, Signature}, base_types::SuiAddress};

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

  pub fn sign<T: Serialize>(&self, msg: &T, intent: Intent) -> Result<Signature> {
    let sig = Signature::new_secure(
      &IntentMessage::new(intent, msg),
      &*self.keypair
    );

    Ok(sig)
  }
}
