use eyre::{Result};
use sui_types::{
  transaction::{GasData, TransactionData}, crypto::Signer,
};
use crate::utils::config::KeyPair;
use super::{
  gas_pool::GasPool, gas_meter::GasMeter,
};

pub struct Sponsor {
  sponsor_keypair: KeyPair,
  gas_pool: GasPool,
  gas_meter: GasMeter,
}

impl Sponsor {
  pub fn new(
    sponsor_keypair: KeyPair,
    gas_pool: GasPool,
    gas_meter: GasMeter,
  ) -> Self {
    Self {
      sponsor_keypair,
      gas_pool,
      gas_meter,
    }
  }

  async fn create_gas_data(&self, tx_data: TransactionData) -> Result<GasData> {
    let pubkey = &self.sponsor_keypair.public();

    let gas_data = GasData {
      payment: vec![self.gas_pool.gas_object().await?],
      owner: pubkey.into(),
      price: self.gas_meter.gas_price().await?,
      budget: self.gas_meter.gas_budget(tx_data).await?,
    };
  
    Ok(gas_data) 
  }

  pub async fn request_gas(&self, tx_data: TransactionData) -> Result<String> {
    let gas_data = self.create_gas_data(tx_data).await?;
    let sig = self.sponsor_keypair.sign(&bincode::serialize(&gas_data)?);
    let sig_str = serde_json::to_string(&sig)?;
    
    Ok(sig_str)
  }
}
