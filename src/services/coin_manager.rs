use std::{sync::Arc, str::FromStr};
use eyre::{eyre, Result, Report};
use sui_sdk::{SuiClient, rpc_types::{SuiTransactionBlockResponseOptions, Coin}};
use shared_crypto::intent::Intent;
use sui_types::{
  base_types::{ObjectID, SuiAddress}, gas_coin::GasCoin, transaction::{Transaction, Command},
  quorum_driver_types::ExecuteTransactionRequestType, programmable_transaction_builder::ProgrammableTransactionBuilder, TypeTag, Identifier
};
use tokio::time::{sleep, Duration};
use crate::{
  storage::{redis::ConnectionPool, redlock::RedLock}, map_err
};
use super::wallet::Wallet;

const GAS_KEY_PREFIX: &str = "gas:";
const MASTER_COIN_KEY: &str = "gas::master_coin";

/// The role of CoinManager is to merge small coins into a single one and the split those into smaller ones.
/// Those smaller coins will be added into the Gas Pool and later consumer by the GasPool service.
/// In essence, this service will make sure that the GasPool has always enough Gas Coins and that the Sponsor account
/// does not have too many dust Gas Coins. More specicifaclly, Gas Coins are used in sponsored transactions and thus
/// their balance is getting low over time. At some point each such Gas coin will be so small that it cannot be used
/// in any sponsored transaction. CoinManager will make sure to clear up those dust coins and recreate big enough coins
/// which are added back to the Gas Pool
pub struct CoinManager {
  api: Arc<SuiClient>,
  wallet: Arc<Wallet>,
  redis_pool: Arc<ConnectionPool>,
  redlock: Arc<RedLock>,
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
    wallet: Arc<Wallet>,
    redis_pool: Arc<ConnectionPool>,
    redlock: Arc<RedLock>,
    max_capacity: usize,
    min_pool_count: usize,
    sponsor: SuiAddress,
  ) -> Self {
    Self {
      api,
      wallet,
      redis_pool,
      redlock,
      max_capacity,
      min_pool_count,
      master_coin: None,
      sponsor
    }
  }

  /// Set the master coin. This will be common for all instances of this service so it has to work in
  /// a distributed environment. That's why we use a distributed lock.
  async fn set_master_coin(&mut self, coins: &mut Vec<ObjectID>) -> Result<()> {
    if let None = self.master_coin {
      // load from redis if exist. Make sure no other service performs the same set of actions
      let lock = self.redlock.lock(
        MASTER_COIN_KEY.as_bytes(),
        Duration::from_secs(10000).as_millis() as usize,
      ).await?;

      let mut conn = self.redis_pool.connection().await?;

      // If there is no master coin in redis then set one of the sponsor's coins
      let Ok(master_coin) = conn.get(MASTER_COIN_KEY).await else {
        let last_index = coins.len() - 1;
        self.master_coin = Some(coins[last_index].clone());

        // Note! We exlcude the master coin from the coins that will
        coins.remove(last_index);

        return Ok(())
      };

      // otherwise use the Redis master coin. This will be shared by all other instances
      self.master_coin = Some(ObjectID::from_hex_literal(&master_coin)?);
      self.redlock.unlock(lock).await;
    }

    Ok(())
  }

  async fn merge_to_master_coin(&self, input_coins: Vec<ObjectID>) -> Result<()> {
    let mut ptb = ProgrammableTransactionBuilder::new();
    let sui_coin_arg_type = map_err!(TypeTag::from_str("0x2::sui::SUI"))?;
    let split_fun = map_err!(Identifier::from_str("split"))?
    
    let merge_coin_command = Command::MergeCoins(
      ptb.obj(ObjectArg::ImmOrOwnedObject())
    );

    let merge_coins = pt_builder.command(merge_coin_command);

    let tx_data = self.api.transaction_builder().pay_all_sui(
      self.sponsor,
      input_coins,
      self.sponsor,
      // TODO: gas meter accepts TransactionData to find the buget. But here we need the budget to construct the tx data
      // in the first place. For the time being we use  a hardcoded value
      100_000
    ).await
    .map_err(|e| Report::msg(e))?;

    let signature = self.wallet.sign(&tx_data)?;

    self.api
    .quorum_driver_api()
    .execute_transaction_block(
      Transaction::from_data(tx_data, Intent::sui_transaction(), vec![signature]).verify()?,
      SuiTransactionBlockResponseOptions::full_content(),
      Some(ExecuteTransactionRequestType::WaitForEffectsCert),
    )
    .await?;

    Ok(())
  }

  async fn fetch_coins(&self) -> Result<Vec<Coin>> {
    let mut coins = vec![];
    let mut cursor = None;

    loop {
      let response = self.api.coin_read_api().get_coins(
        self.sponsor,
        Some(GasCoin::type_().to_canonical_string()),
        cursor,
        None,
      )
      .await?;

      coins.extend(response.data);

      if !response.has_next_page {break}
      cursor = response.next_cursor;
    }

    Ok(coins)
  }

  pub async fn execute(&mut self, current_coins: Vec<String>) -> Result<()> {
    // 1. Load all coins that belong to the sponsor account
    let coins = self.fetch_coins()
    .await?
    .into_iter()
    .map(|c| c.coin_object_id)
    .collect::<Vec<_>>();
    

    // 2. Exclude the ones that are currently in the Gas Pool
    let mut input_coins = coins.into_iter()
    .filter(|c| !current_coins.contains(&c.to_hex_literal()))
    .collect::<Vec<_>>();
    
    // 3. Set the master coin if needed.
    self.set_master_coin(&mut input_coins).await?;

    let mut pt_builder = ProgrammableTransactionBuilder::new();
    
    // 4. Merge all these coins into the master coin
    self.merge_to_master_coin(input_coins).await?;
    

    // 5 Split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins

    // 6. Store the new coins into the pool; one Redis entry for each object id
    Ok(())
  }

  async fn get_pool_coins(&self) -> Result<Vec<String>> {
    // check the numbet of Gas coins in the pool
    let mut conn = self.redis_pool.connection().await?;
    let gas_coins = conn.keys(GAS_KEY_PREFIX).await?;

    Ok(gas_coins)
  }

  /// A loop that periodically checks if the number of Gas coins in the pool is lower than our capacity
  pub async fn run(&mut self) -> Result<()> {
    loop {
      let pool_coins = self.get_pool_coins().await?;

      if pool_coins.len() <= self.min_pool_count {
        self.execute(pool_coins).await?;
      }
      
      sleep(Duration::from_secs(1)).await;
    }
  }
}
