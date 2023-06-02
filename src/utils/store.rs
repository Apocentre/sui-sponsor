use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::services::sponsor::{Sponsor};
use super::config::{Config};

pub struct Store {
  pub config: Config,
  pub rpc_client: SuiClient,
  pub sponsor: Sponsor,
}

impl  Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let rpc_client = SuiClientBuilder::default()
    .build(&config.sui.rpc)
    .await.unwrap();

    let sponsor = Sponsor::new(
      config.sui.sponsor_priv_key.clone(),
      config.sui.sponsor_address.0
    );

    Self {
      config,
      rpc_client,
      sponsor,
    }
  }
}
