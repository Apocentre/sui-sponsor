use std::str::FromStr;
use envconfig::Envconfig;
use sui_types::base_types;
use eyre::Report;

#[derive(Envconfig)]
pub struct Config {
  #[envconfig(from = "PORT")]
  pub port: u64,
  #[envconfig(from = "CORS_ORIGIN")]
  pub cors_config: CorsConfig,
  #[envconfig(nested = true)]
  pub sui: SuiConfig,
  #[envconfig(from = "FIREBASE_API_KEY")]
  pub firebase_api_key: String,
}

#[derive(Envconfig)]
pub struct SuiConfig {
  #[envconfig(from = "SUI_RPC")]
  pub rpc: String,
  #[envconfig(from = "SPONSOR_PRIV_KEY")]
  pub sponsor_priv_key: String,
  #[envconfig(from = "SPONSOR_ADDRESS")]
  pub sponsor_address: SuiAddress,
}

pub struct SuiAddress(base_types::SuiAddress);

impl FromStr for SuiAddress {
  type Err = Report;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let addr = base_types::SuiAddress::from_str(s)
    .map_err(|e| Report::msg(e.to_string()))?;

    Ok(Self(addr))
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

