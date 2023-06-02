use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::services::{sponsor::Sponsor, gas_meter::GasMeter};
use super::config::{Config};

pub struct Store {
  pub config: Config,
  pub rpc_client: SuiClient,
  pub sponsor: Sponsor,
}

impl Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let rpc_client = SuiClientBuilder::default()
    .build(&config.sui.rpc)
    .await.unwrap();

    let gas_meter = GasMeter::new();
    let sponsor = Sponsor::new(
      config.sui.sponsor_keypair.clone(),
      gas_meter,
    );

    Self {
      config,
      rpc_client,
      sponsor,
    }
  }
}
