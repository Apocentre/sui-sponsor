use std::sync::Arc;
use envconfig::Envconfig;
use clap::{Parser};
use sui_sdk::{SuiClientBuilder, SuiClient};
use crate::{
  services::{sponsor::Sponsor, gas_meter::GasMeter, wallet::Wallet},
  gas_pool::{GasPool, coin_object_producer::CoinObjectProducer},
  storage::{redis::ConnectionPool, redlock::RedLock}
};
use super::config::{Config};

#[derive(Debug, Parser)]
struct Args {
  /// The metadata consumer tag
  #[arg(short = 'c', long, default_value_t = String::from("coin_object"))]
  consumer_tag: String,
}


pub struct Store {
  pub config: Config,
  pub rpc_client: Arc<SuiClient>,
  pub wallet: Arc<Wallet>,
  pub gas_meter: Arc<GasMeter>,
  pub sponsor: Sponsor,
  pub redis_pool: Arc<ConnectionPool>,
  pub redlock: Arc<RedLock>,
  pub coin_object_producer: Arc<CoinObjectProducer>,
}

impl Store {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let args = Args::parse();

    let rpc_client = Arc::new(
      SuiClientBuilder::default()
      .build(&config.sui.rpc)
      .await.unwrap()
    );

    let redis_pool = Arc::new(ConnectionPool::new(&config.redis.host, &config.redis.password, config.redis.port));
    let redlock = Arc::new(RedLock::new(vec![&config.redis.host], &config.redis.password));

    let coin_object_producer = Arc::new(
      CoinObjectProducer::try_new(
        config.rabbitmq.uri.clone(),
        config.rabbitmq.retry_ttl
      ).await.expect("create coin object producer")
    );

    let wallet = Arc::new(Wallet::new(config.sui.sponsor_keypair.clone()));
    let gas_pool = GasPool::try_new(
      Arc::clone(&rpc_client),
      Arc::clone(&redis_pool),
      &config.rabbitmq.uri,
      &args.consumer_tag,
      Arc::clone(&coin_object_producer)
    ).await;
    let gas_meter = Arc::new(GasMeter::new(Arc::clone(&rpc_client)));

    let sponsor = Sponsor::new(
      Arc::clone(&rpc_client),
      Arc::clone(&wallet),
      Arc::clone(&gas_meter),
      gas_pool,
      config.gas_pool.min_coin_balance.unwrap(),
    );

    Self {
      config,
      rpc_client: rpc_client,
      wallet,
      gas_meter,
      sponsor,
      redis_pool,
      redlock,
      coin_object_producer,
    }
  }
}
