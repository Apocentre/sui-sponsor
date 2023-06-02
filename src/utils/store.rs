use std::sync::Arc;
use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::{
  services::{sponsor::Sponsor, gas_meter::GasMeter, gas_pool::GasPool, wallet::Wallet}, 
  storage::{redis::ConnectionPool, redlock::RedLock}
};
use super::config::{Config};

pub struct Store {
  pub config: Config,
  pub rpc_client: Arc<SuiClient>,
  pub sponsor: Sponsor,
  pub redis_pool: Arc<ConnectionPool>,
  pub redlock: Arc<RedLock>,
}

impl Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let rpc_client = Arc::new(
      SuiClientBuilder::default()
      .build(&config.sui.rpc)
      .await.unwrap()
    );

    let redis_pool = Arc::new(ConnectionPool::new(&config.redis.host, &config.redis.password, config.redis.port));
    let redlock = Arc::new(RedLock::new(vec![&config.redis.host], &config.redis.password));

    let wallet = Arc::new(Wallet::new(config.sui.sponsor_keypair.clone()));
    let gas_pool = GasPool::new(Arc::clone(&rpc_client));
    let gas_meter = GasMeter::new(Arc::clone(&rpc_client));
    let sponsor = Sponsor::new(
      Arc::clone(&wallet),
      gas_pool,
      gas_meter,
    );

    Self {
      config,
      rpc_client: rpc_client,
      sponsor,
      redis_pool,
      redlock,
    }
  }
}
