use std::sync::Arc;
use tokio::sync::Mutex;
use envconfig::Envconfig;
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::{
  services::{sponsor::Sponsor, gas_meter::GasMeter, gas_pool::GasPool, wallet::Wallet, coin_manager::CoinManager}, 
  storage::{redis::ConnectionPool, redlock::RedLock}
};
use super::config::{Config};

pub struct Store {
  pub config: Config,
  pub rpc_client: Arc<SuiClient>,
  pub sponsor: Sponsor,
  pub redis_pool: Arc<ConnectionPool>,
  pub redlock: Arc<RedLock>,
  pub coin_manager: Arc<Mutex<CoinManager>>,
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
    let sponsor_address = wallet.address();
    let gas_pool = GasPool::new(Arc::clone(&rpc_client));
    let gas_meter = Arc::new(GasMeter::new(Arc::clone(&rpc_client)));
    let sponsor = Sponsor::new(
      Arc::clone(&wallet),
      Arc::clone(&gas_meter),
      gas_pool,
    );

    let coin_manager = Arc::new(Mutex::new(CoinManager::new(
      Arc::clone(&rpc_client),
      Arc::clone(&wallet),
      Arc::clone(&gas_meter),
      Arc::clone(&redis_pool),
      Arc::clone(&redlock),
      config.gas_pool.max_capacity,
      config.gas_pool.min_pool_count,
      config.gas_pool.min_coin_balance,
      sponsor_address,
    )));

    Self {
      config,
      rpc_client: rpc_client,
      sponsor,
      redis_pool,
      redlock,
      coin_manager,
    }
  }
}
