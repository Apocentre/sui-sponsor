use std::{sync::Arc, str::FromStr, collections::HashSet};
use eyre::Result;
use sui_sdk::SuiClient;
use sui_types::{base_types::{ObjectID, SuiAddress}, gas_coin::GasCoin};
use tokio::time::{sleep, Duration};
use crate::storage::redis::ConnectionPool;

/// The role of CoinManager is to merge small coins into a single one and the split those into smaller ones.
/// Those smaller coins will be added into the Gas Pool and later consumer by the GasPool service.
/// In essence, this service will make sure that the GasPool has always enough Gas Coins and that the Sponsor account
/// does not have too many dust Gas Coins. More specicifaclly, Gas Coins are used in sponsored transactions and thus
/// their balance is getting low over time. At some point each such Gas coin will be so small that it cannot be used
/// in any sponsored transaction. CoinManager will make sure to clear up those dust coins and recreate big enough coins
/// which are added back to the Gas Pool
pub struct CoinManager {
  api: Arc<SuiClient>,
  redis_pool: Arc<ConnectionPool>,
  max_capacity: usize,
  min_pool_count: usize,
  /// This is the coin that we all other coins will be merged into. We will select one of Sponsor's coins during the first
  /// run of the gas coin creation logic below.
  master_coin: Option<ObjectID>,
  sponsor: SuiAddress,
}

impl CoinManager {
  pub fn new(
    api: Arc<SuiClient>,
    redis_pool: Arc<ConnectionPool>,
    max_capacity: usize,
    min_pool_count: usize,
    sponsor: SuiAddress,
  ) -> Self {
    Self {
      api,
      redis_pool,
      max_capacity,
      min_pool_count,
      master_coin: None,
      sponsor
    }
  }

  pub async fn execute(&self, current_coins: Vec<String>) -> Result<()> {
    // 1. Load all coins that belong to the sponsor account
    let coins = self.api.coin_read_api().get_coins(
      self.sponsor,
      Some(GasCoin::type_().to_canonical_string()),
      None,
      None,
    )
    .await?
    .data
    .into_iter()
    .map(|c| c.coin_object_id)
    .collect::<HashSet<_>>();

    // 2. Exclude the ones that are currently in the Gas Pool
    let object_ids = coins.into_iter()
    .filter(|c| !current_coins.contains(&c.to_hex_literal()))
    .collect::<Vec<_>>();
    
    // 2. Merge all these coins into one single coin which is the so called master coin
    

    // 3. Split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins

    // 4. Store the new coins into the pool; one Redis entry for each object id
    Ok(())
  }

  async fn get_pool_coins(&self) -> Result<Vec<String>> {
    // check the numbet of Gas coins in the pool
    let mut conn = self.redis_pool.connection().await?;
    let gas_coins = conn.keys("gas:").await?;

    Ok(gas_coins)
  }

  /// A loop that periodically checks if the number of Gas coins in the pool is lower than our capacity
  pub async fn run(&self) -> Result<()> {
    loop {
      let pool_coins = self.get_pool_coins().await?;

      if pool_coins.len() <= self.min_pool_count {
        self.execute(pool_coins).await?;
      }
      
      sleep(Duration::from_secs(1)).await;
    }
  }
}


