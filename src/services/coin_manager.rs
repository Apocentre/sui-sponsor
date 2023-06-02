use std::sync::Arc;
use eyre::Result;
use sui_sdk::SuiClient;

/// The role of CoinManager is to merge small coins into a single one and the split those into smaller ones.
/// Those smaller coins will be added into the Gas Pool and later consumer by the GasPool service.
/// In essence, this service will make sure that the GasPool has always enough Gas Coins and that the Sponsor account
/// does not have too many dust Gas Coins. More specicifaclly, Gas Coins are used in sponsored transactions and thus
/// their balance is getting low over time. At some point each such Gas coin will be so small that it cannot be used
/// in any sponsored transaction. CoinManager will make sure to clear up those dust coins and recreate big enough coins
/// which are added back to the Gas Pool
pub struct CoinManager {
  api: Arc<SuiClient>
}

impl CoinManager {
  pub fn new(api: Arc<SuiClient>) -> Self {
    Self {api}
  }

  pub fn execute() -> Result<()> {
    // 1. Load all coins from the pool. We load this from Redis

    // 2. Merge all these coins into one single coin which is the so called master coin

    // 3. Split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins

    // 4. Store the new coins into the pool; one Redis entry for each object id
    Ok(())
  }
}


