use std::sync::Arc;
use eyre::{Result};
use shared_crypto::intent::Intent;
use sui_types::{
  transaction::{GasData, TransactionData},
};
use crate::gas_pool::GasPool;
use super::{
  gas_meter::GasMeter, wallet::Wallet,
};

pub struct Sponsor {
  wallet: Arc<Wallet>,
  gas_meter: Arc<GasMeter>,
  gas_pool: GasPool,
}

impl Sponsor {
  pub fn new(
    wallet: Arc<Wallet>,
    gas_meter: Arc<GasMeter>,
    gas_pool: GasPool,
  ) -> Self {
    Self {
      wallet,
      gas_pool,
      gas_meter,
    }
  }

  async fn create_gas_data(&self, tx_data: TransactionData) -> Result<GasData> {
    let pubkey = &self.wallet.public();

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
    let sig = self.wallet.sign(&gas_data, Intent::sui_transaction())?;
    let sig_str = serde_json::to_string(&sig)?;
    
    Ok(sig_str)
  }
}
