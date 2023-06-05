use std::sync::Arc;
use shared_crypto::intent::Intent;
use eyre::Result;
use sui_sdk::{
  rpc_types::{
    SuiTransactionBlockResponse, SuiTransactionBlockEffects, SuiExecutionStatus, SuiTransactionBlockResponseOptions
  },
  SuiClient,
};
use sui_types::{
  transaction::{Transaction, TransactionData}, quorum_driver_types::ExecuteTransactionRequestType,
  crypto::Signature,
};

pub struct TxManager {
  api: Arc<SuiClient>,
}

impl TxManager {
  pub fn new(api: Arc<SuiClient>) -> Self {
    Self {api}
  }

  pub fn has_errors(response: &SuiTransactionBlockResponse) -> bool {
    if response.errors.len() > 0 {return true}

    if let Some(effects) = response.effects.as_ref() {
      let SuiTransactionBlockEffects::V1(effects) = effects;
      
      if let SuiExecutionStatus::Failure {..} = effects.status {
        return true
      } 
    }

    false
  }

  pub async fn send_tx(
    &self,
    tx_data: TransactionData,
    signatures: Vec<Signature>
  ) -> Result<SuiTransactionBlockResponse> {
    let response = self.api
    .quorum_driver_api()
    .execute_transaction_block(
      Transaction::from_data(tx_data, Intent::sui_transaction(), signatures).verify()?,
      SuiTransactionBlockResponseOptions::full_content(),
      Some(ExecuteTransactionRequestType::WaitForLocalExecution),
    )
    .await
    .expect("successul rebalancing");

    Ok(response)
  }
}