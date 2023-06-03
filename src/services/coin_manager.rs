use std::{sync::Arc};
use eyre::{eyre, Result};
use shared_crypto::intent::Intent;
use sui_sdk::{SuiClient, rpc_types::{Coin, SuiTransactionBlockResponseOptions}};
use sui_types::{
  base_types::{ObjectID, SuiAddress, ObjectRef}, gas_coin::GasCoin, transaction::{Command, ObjectArg, TransactionData, Transaction},
  programmable_transaction_builder::ProgrammableTransactionBuilder, quorum_driver_types::ExecuteTransactionRequestType,
};
use tokio::time::{sleep, Duration};
use crate::{
  storage::{redis::ConnectionPool, redlock::RedLock}, map_err,
  helpers::object::get_object_ref,
};
use super::{wallet::Wallet, gas_meter::GasMeter};

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
  gas_meter: Arc<GasMeter>,
  redis_pool: Arc<ConnectionPool>,
  redlock: Arc<RedLock>,
  max_capacity: usize,
  min_pool_count: usize,
  // The minimum balance each coin that is created and added to the Gas Pool should have
  coin_balance: u64,
  /// This is the coin that we all other coins will be merged into. We will select one of Sponsor's coins during the first
  /// run of the gas coin creation logic below.
  master_coin: Option<ObjectID>,
  sponsor: SuiAddress,
}

impl CoinManager {
  pub fn new(
    api: Arc<SuiClient>,
    wallet: Arc<Wallet>,
    gas_meter: Arc<GasMeter>,
    redis_pool: Arc<ConnectionPool>,
    redlock: Arc<RedLock>,
    max_capacity: usize,
    min_pool_count: usize,
    coin_balance: u64,
    sponsor: SuiAddress,
  ) -> Self {
    Self {
      api,
      wallet,
      gas_meter,
      redis_pool,
      redlock,
      max_capacity,
      min_pool_count,
      coin_balance,
      master_coin: None,
      sponsor
    }
  }

  /// Set the master coin. This will be common for all instances of this service so it has to work in
  /// a distributed environment. That's why we use a distributed lock.
  async fn set_master_coin(&mut self, sponsor_coins: &mut Vec<ObjectRef>) -> Result<()> {
    // when the service start there is no in-memory master coin
    if let None = self.master_coin {
      // load from redis if exist. Make sure no other service performs the same set of actions
      let lock = self.redlock.lock(
        MASTER_COIN_KEY.as_bytes(),
        Duration::from_secs(10000).as_millis() as usize,
      ).await?;

      let mut conn = self.redis_pool.connection().await?;

      // If there is no master coin in redis then set the biggest coin from the sponsor's coins set
      // The biggest coins in the first item in the list
      let Ok(master_coin) = conn.get(MASTER_COIN_KEY).await else {
        self.master_coin = Some(sponsor_coins.first().expect("sponsor to have coins").clone().0);

        // Note! We exlcude the master coin from the coins that will
        sponsor_coins.remove(0);

        return Ok(())
      };

      // otherwise use the Redis master coin. This will be shared by all other instances
      self.master_coin = Some(ObjectID::from_hex_literal(&master_coin)?);
      self.redlock.unlock(lock).await;
    }

    Ok(())
  }

  /// It will first merge all user coins (except for those that are still in the Gas Pool) into the master coin.
  /// Then it split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins; thus rebalancing
  /// Sponsor's coins and keeping Gas Pool liquid.
  async fn rebalance_coins(&self, input_coins: Vec<ObjectRef>, gas_pool_coin_count: usize) -> Result<()> {
    let mut ptb = ProgrammableTransactionBuilder::new();
    let master_coin_obj_ref = get_object_ref(Arc::clone(&self.api), self.master_coin.unwrap()).await?;

    // 1. Merge all these coins into the master coin 
    // If the sponsor has only one coin the input_coins (which exclude the master coin) will be empty and thus
    // we can skip the merge step in this iteration.
    if input_coins.len() > 0 {
      let input_coin_args = input_coins.into_iter()
      .map(|c| ptb.obj(ObjectArg::ImmOrOwnedObject(c)).expect("coin object ref"))
      .collect::<Vec<_>>();

      let merge_coin_cmd = Command::MergeCoins(
        map_err!(ptb.obj(ObjectArg::ImmOrOwnedObject(master_coin_obj_ref)))?,
        input_coin_args,
      );
      ptb.command(merge_coin_cmd);
    }
    // 2. Split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT each having `coin_balance`
    let amounts = vec![self.coin_balance; self.max_capacity - gas_pool_coin_count]
    .into_iter()
    .map(|a| ptb.pure(a).expect("pure arg"))
    .collect::<Vec<_>>();

    let split_coin_cmd = Command::SplitCoins(
      map_err!(ptb.obj(ObjectArg::ImmOrOwnedObject(master_coin_obj_ref)))?,
      amounts,
    );
    ptb.command(split_coin_cmd);
    
    let pt = ptb.finish();
    let tx_data = TransactionData::new_programmable(
      self.sponsor,
      vec![master_coin_obj_ref],
      pt,
      100_000,
      self.gas_meter.gas_price().await?,
    );

    let response = self.api
    .read_api()
    .dry_run_transaction_block(tx_data.clone())
    .await?;

    println!(">>>>>>>>>>>>> {:?}", response);

    let signature = self.wallet.sign(&tx_data)?;
    let transaction_response = self.api
    .quorum_driver_api()
    .execute_transaction_block(
      Transaction::from_data(tx_data, Intent::sui_transaction(), vec![signature]).verify()?,
      SuiTransactionBlockResponseOptions::full_content(),
      Some(ExecuteTransactionRequestType::WaitForEffectsCert),
    )
    .await
    .expect("successul rebalancing");

    println!(">>>>>>>> {:?}", transaction_response);

    Ok(())
  }

  /// Fetches all coins that belong to the sponsor. It will return a sorted array of coins according to their balance
  async fn fetch_coins(&self) -> Result<Vec<Coin>> {
    let mut coins = vec![];
    let mut cursor = None;

    println!(">>>>>>>>> {:?}", self.sponsor);

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

    coins.sort_by(|a, b| b.balance.cmp(&a.balance));
    Ok(coins)
  }

  /// Main execution logic
  pub async fn execute(&mut self, current_coins: Vec<String>) -> Result<()> {
    // 1. Load all coins that belong to the sponsor account
    let coins = self.fetch_coins()
    .await?
    .into_iter()
    .map(|c| c.object_ref())
    .collect::<Vec<_>>();

    // TODO: We might want to send a notification i.e. email instead of panicking 
    if coins.len() == 0 {
      panic!("Sponsor MUST have at least one coin");
    }
    
    let gas_pool_coin_count = current_coins.len();

    // 2. Exclude the ones that are currently in the Gas Pool
    let mut input_coins = coins.into_iter()
    .filter(|(c, _, _)| !current_coins.contains(&c.to_hex_literal()))
    .collect::<Vec<_>>();
    
    // 3. Set the master coin if needed.
    self.set_master_coin(&mut input_coins).await?;
    
    // 4. Rebalance coins
    self.rebalance_coins(input_coins, gas_pool_coin_count).await?;
    
    // 6. TODO: Store the new coins into the pool; one Redis entry for each object id
    
    Ok(())
  }

  /// read the gas pool coins from Redis
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
