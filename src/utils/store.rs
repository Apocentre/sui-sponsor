use super::config::Config;
use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};

pub struct Store {
  pub config: Config,
  pub rpc_client: SuiClient,
}

impl Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let rpc_client = SuiClientBuilder::default()
    .build(&config.sui.rpc)
    .await.unwrap();

    Self {
      config,
      rpc_client,
    }
  }
}
