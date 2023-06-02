use std::sync::Arc;
use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::services::{sponsor::Sponsor, gas_meter::GasMeter};
use super::config::{Config};

pub struct Store {
  pub config: Config,
  pub rpc_client: Arc<SuiClient>,
  pub sponsor: Sponsor,
}

impl Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let rpc_client = Arc::new(
      SuiClientBuilder::default()
      .build(&config.sui.rpc)
      .await.unwrap()
    );

    let gas_meter = GasMeter::new(Arc::clone(&rpc_client));
    let sponsor = Sponsor::new(
      config.sui.sponsor_keypair.clone(),
      gas_meter,
    );

    Self {
      config,
      rpc_client: rpc_client,
      sponsor,
    }
  }
}
