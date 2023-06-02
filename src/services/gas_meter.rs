use std::sync::Arc;
use eyre::Result;
use sui_sdk::{SuiClient, rpc_types::SuiTransactionBlockEffects};
use sui_types::{transaction::TransactionData, gas::GasCostSummary};

pub struct GasMeter {
  api: Arc<SuiClient>,
}

impl GasMeter {
  pub fn new(api: Arc<SuiClient>) -> Self {
    Self {api}
  }

  pub async fn gas_price(&self) -> Result<u64> {
    let gas = self.api.read_api()
    .get_reference_gas_price()
    .await?;

    Ok(gas)
  }

  pub async fn gas_budget(&self, tx_data: TransactionData) -> Result<u64> {
    let tx_block_response = self.api.read_api()
    .dry_run_transaction_block(tx_data)
    .await?;

    Ok(Self::total_gas_used_upper_bound(tx_block_response.effects)?)
  }

  pub fn total_gas_used(tx_block_effects: SuiTransactionBlockEffects) -> Result<u64> {
    let gas_summary = Self::gas_summary(tx_block_effects);
    let gas_used = gas_summary.computation_cost
    + gas_summary.storage_cost
    - gas_summary.storage_rebate;

    Ok(gas_used)
  }

  pub fn total_gas_used_upper_bound(tx_block_effects: SuiTransactionBlockEffects) -> Result<u64> {
    let gas_summary = Self::gas_summary(tx_block_effects);
    // Note for the upper bound we don't subtract`storage_rebate`. This is similar to how the TS SDK computes this value
    let gas_upper_boud = gas_summary.computation_cost + gas_summary.storage_cost;
    
    Ok(gas_upper_boud)
  }

  fn gas_summary(tx_block_effects: SuiTransactionBlockEffects) -> GasCostSummary {
    match tx_block_effects {
      SuiTransactionBlockEffects::V1(effects_v1) => effects_v1.gas_used,
    }
  }
}
