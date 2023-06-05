use std::{str::FromStr, sync::Arc, ops::Deref};
use envconfig::Envconfig;
use sui_types::{crypto::SuiKeyPair};
use eyre::Report;

#[derive(Envconfig)]
pub struct Config {
  #[envconfig(from = "PORT")]
  pub port: Option<u64>,
  #[envconfig(from = "CORS_ORIGIN")]
  pub cors_config: Option<CorsConfig>,
  #[envconfig(nested = true)]
  pub sui: SuiConfig,
  #[envconfig(nested = true)]
  pub redis: RedisConfig,
  #[envconfig(nested = true)]
  pub rabbitmq: RabbitMQConfig,
  #[envconfig(nested = true)]
  pub gas_pool: GasPoolConfig,
  #[envconfig(from = "FIREBASE_API_KEY")]
  pub firebase_api_key: Option<String>,
}

#[derive(Envconfig)]
pub struct GasPoolConfig {
  #[envconfig(from = "MAX_POOL_CAPACITY")]
  pub max_capacity: Option<usize>,
  #[envconfig(from = "MIN_POOL_COUNT")]
  pub min_pool_count: Option<usize>,
  #[envconfig(from = "COIN_BALANCE_DEPOSIT")]
  pub coin_balance: Option<u64>,
  #[envconfig(from = "MIN_COIN_BALANCE")]
  pub min_coin_balance: Option<u64>,
}

#[derive(Envconfig)]
pub struct RabbitMQConfig {
  #[envconfig(from = "RABBITMQ_URI")]
  pub uri: String,
  #[envconfig(from = "RETRY_TTL")]
  pub retry_ttl: u32,
}

#[derive(Envconfig)]
pub struct SuiConfig {
  #[envconfig(from = "SUI_RPC")]
  pub rpc: String,
  #[envconfig(from = "SPONSOR_PRIV_KEY")]
  pub sponsor_keypair: KeyPair,
}

#[derive(Envconfig)]
pub struct RedisConfig {
  #[envconfig(from = "REDIS_HOST")]
  pub host: String,
  #[envconfig(from = "REDIS_PORT")]
  pub port: u16,
  #[envconfig(from = "REDIS_PASSWORD")]
  pub password: String,
}

pub struct KeyPair(Arc<SuiKeyPair>);

impl FromStr for KeyPair {
  type Err = Report;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let keypair = SuiKeyPair::from_str(s)
    .map_err(|e| Report::msg(e.to_string()))?;

    Ok(Self(Arc::new(keypair)))
  }
}

impl Deref for KeyPair {
  type Target = SuiKeyPair;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl Clone for KeyPair {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

pub struct CorsConfig {
  pub origin: Vec<String>,
}

impl FromStr for CorsConfig {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(Self {
      origin: s.split(",").map(|val| val.to_owned()).collect(),
    })
  }
}

