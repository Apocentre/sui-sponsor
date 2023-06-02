use std::sync::Arc;
use eyre::Result;
use sui_sdk::SuiClient;

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

  pub fn gas_budget(&self) -> u64 {
    // TODO: Compute gas estimation
    0
  }
}
