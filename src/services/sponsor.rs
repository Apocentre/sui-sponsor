use std::sync::{Arc};
use eyre::{Result};
use sui_types::{
  transaction::{GasData}, base_types::{SuiAddress},
};
use super::gas_pool::{get_gas_object};

pub struct Sponsor {
  sponsor_priv_key: String,
  sponsor_address: SuiAddress,
}

impl  Sponsor {
  pub fn new(sponsor_priv_key: String, sponsor_address: SuiAddress) -> Self {
    Self {sponsor_priv_key, sponsor_address}
  }

  pub fn request_gas(&self) -> Result<GasData> {
    let gas_data =  GasData {
      payment: vec![get_gas_object()?],
      owner: self.sponsor_address,
      price: todo!(),
      budget: todo!(),
    };
  
    Ok(gas_data)  
  }
}
