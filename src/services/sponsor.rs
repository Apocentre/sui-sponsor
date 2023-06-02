use eyre::{Result};
use sui_types::{
  transaction::GasData, crypto::Signer,
};
use crate::utils::config::KeyPair;
use super::{
  gas_pool::{get_gas_object}, gas_meter::GasMeter,
};

pub struct Sponsor {
  sponsor_keypair: KeyPair,
  gas_meter: GasMeter,
}

impl Sponsor {
  pub fn new(sponsor_keypair: KeyPair, gas_meter: GasMeter) -> Self {
    Self {
      sponsor_keypair,
      gas_meter,
    }
  }

  async fn create_gas_data(&self) -> Result<GasData> {
    let pubkey = &self.sponsor_keypair.public();

    let gas_data = GasData {
      payment: vec![get_gas_object()?],
      owner: pubkey.into(),
      price: self.gas_meter.gas_price().await?,
      budget: self.gas_meter.gas_budget(),
    };
  
    Ok(gas_data) 
  }

  pub async fn request_gas(&self) -> Result<String> {
    let gas_data = self.create_gas_data().await?;
    let sig = self.sponsor_keypair.sign(&bincode::serialize(&gas_data)?);
    let sig_str = serde_json::to_string(&sig)?;
    
    Ok(sig_str)
  }
}
