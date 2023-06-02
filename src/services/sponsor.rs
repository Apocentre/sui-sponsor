use eyre::{Result};
use sui_types::{
  transaction::{GasData}, base_types::{SuiAddress},
};
use super::{
  gas_pool::{get_gas_object}, gas_meter::{GasMeter},
};

pub struct Sponsor {
  sponsor_priv_key: String,
  sponsor_address: SuiAddress,
  gas_meter: GasMeter,
}

impl  Sponsor {
  pub fn new(
    sponsor_priv_key: String,
    sponsor_address: SuiAddress,
    gas_meter: GasMeter,
  ) -> Self {
    Self {
      sponsor_priv_key,
      sponsor_address,
      gas_meter,
    }
  }

  pub fn request_gas(&self) -> Result<GasData> {
    let gas_data =  GasData {
      payment: vec![get_gas_object()?],
      owner: self.sponsor_address,
      price: self.gas_meter.gas_price(),
      budget: self.gas_meter.gas_budget(),
    };
  
    Ok(gas_data)  
  }
}
