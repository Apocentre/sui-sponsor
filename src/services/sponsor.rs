use eyre::Result;
use sui_types::transaction::GasData;
use super::gas_pool::{get_gas_object};

pub fn request_gas() -> Result<GasData> {
  let gas_data =  GasData {
    payment: vec![get_gas_object()],
    owner: 
  };

  Ok(gas_data)  
}
