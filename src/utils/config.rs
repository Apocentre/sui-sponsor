use std::{str::FromStr, sync::Arc, ops::Deref};
use envconfig::Envconfig;
use sui_types::{crypto::SuiKeyPair};
use eyre::Report;

#[derive(Envconfig)]
pub struct Config {
  #[envconfig(from = "PORT")]
  pub port: u64,
  #[envconfig(from = "CORS_ORIGIN")]
  pub cors_config: CorsConfig,
  #[envconfig(nested = true)]
  pub sui: SuiConfig,
  #[envconfig(nested = true)]
  pub redis: RedisConfig,
  #[envconfig(from = "FIREBASE_API_KEY")]
  pub firebase_api_key: String,
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

