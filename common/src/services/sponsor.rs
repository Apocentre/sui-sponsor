use std::sync::Arc;
use eyre::{eyre, Result};
use shared_crypto::intent::Intent;
use sui_sdk::{SuiClient};
use sui_types::{
  transaction::{GasData, TransactionData}, base_types::ObjectID, gas_coin::GasCoin,
};
use crate::{gas_pool::GasPool, helpers::object::get_object, map_err};
use super::{
  gas_meter::GasMeter, wallet::Wallet,
};


pub struct Sponsor {
  api: Arc<SuiClient>,
  wallet: Arc<Wallet>,
  gas_meter: Arc<GasMeter>,
  gas_pool: GasPool,
  min_coin_balance: u64,
}

impl Sponsor {
  pub fn new(
    api: Arc<SuiClient>,
    wallet: Arc<Wallet>,
    gas_meter: Arc<GasMeter>,
    gas_pool: GasPool,
    min_coin_balance: u64,
  ) -> Self {
    Self {
      api,
      wallet,
      gas_pool,
      gas_meter,
      min_coin_balance,
    }
  }

  async fn create_gas_data(&mut self, tx_data: TransactionData) -> Result<GasData> {
    let pubkey = &self.wallet.public();

    let gas_data = GasData {
      payment: vec![self.gas_pool.gas_object().await?],
      owner: pubkey.into(),
      price: self.gas_meter.gas_price().await?,
      budget: self.gas_meter.gas_budget(tx_data).await?,
    };
  
    Ok(gas_data) 
  }

  pub async fn gas_object_processed(&mut self, coin_object_id: ObjectID) -> Result<()> {
    let coin = &get_object(Arc::clone(&self.api), coin_object_id).await?;
    let coin_balance = map_err!(TryInto::<GasCoin>::try_into(coin))?;

    // check if the coin_object_id has enough balance. If not then remove it from the queue i.e. ack
    // as well as, from Redis.
    if coin_balance.value() <= self.min_coin_balance {
      self.gas_pool.remove_gas_object(coin_object_id).await?;
    } else {
      self.gas_pool.return_gas_object(coin_object_id).await?;
    }

    Ok(())
  }

  pub async fn request_gas(&mut self, tx_data: TransactionData) -> Result<String> {
    let gas_data = self.create_gas_data(tx_data).await?;
    let sig = self.wallet.sign(&gas_data, Intent::sui_transaction())?;
    let sig_str = serde_json::to_string(&sig)?;
    
    Ok(sig_str)
  }
}
