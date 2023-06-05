use std::{sync::Arc, str::FromStr};
use eyre::{Result, ensure, eyre, ContextCompat};
use shared_crypto::intent::Intent;
use sui_sdk::{
  SuiClient,
  rpc_types::{
    Coin, SuiTransactionBlockResponseOptions, SuiTransactionBlockResponse, SuiTransactionBlockEffects,
    SuiExecutionStatus,
  },
};
use sui_types::{
  base_types::{SuiAddress, ObjectID}, transaction::{Command, ObjectArg, TransactionData, Transaction},
  programmable_transaction_builder::ProgrammableTransactionBuilder, quorum_driver_types::ExecuteTransactionRequestType, Identifier, SUI_FRAMEWORK_PACKAGE_ID, coin, TypeTag,
};
use log::info;
use tokio::time::{sleep, Duration};
use crate::{
  storage::{redis::ConnectionPool}, map_err, helpers::object::get_created_objects,
  gas_pool::coin_object_producer::CoinObjectProducer,
};
use super::{wallet::Wallet, gas_meter::GasMeter};

const GAS_KEY_PREFIX: &str = "gas:";
// This is roughly how much we need to split into 100 coins for the first time.
// Here is an example https://suiexplorer.com/txblock/BMU7d8QJpRQQ9oXZkUPGufUHsfZcc1tWaKtBpCkWjDBC?network=devnet.
// Subsequent calls will require way lower gas because there is a storage rebate from merging coins into one. Here
// is an example of a subsequent tx https://suiexplorer.com/txblock/6SrtMgLUwRv1Xw8YqGmmHHv8c6EVxYnABQXQW5CNSyfq?network=devnet
const GAS_BUDGET: u64 = 150_000_000;

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
  coin_object_producer: Arc<CoinObjectProducer>,
  max_capacity: usize,
  min_pool_count: usize,
  // The minimum balance each coin that is created and added to the Gas Pool should have
  coin_balance: u64,
  sponsor: SuiAddress,
}

impl CoinManager {
  pub fn new(
    api: Arc<SuiClient>,
    wallet: Arc<Wallet>,
    gas_meter: Arc<GasMeter>,
    redis_pool: Arc<ConnectionPool>,
    coin_object_producer: Arc<CoinObjectProducer>,
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
      coin_object_producer,
      max_capacity,
      min_pool_count,
      coin_balance,
      sponsor
    }
  }

  /// read the gas pool coins from Redis
  async fn get_pool_coins(&self) -> Result<Vec<String>> {
    // check the numbet of Gas coins in the pool
    let mut conn = self.redis_pool.connection().await?;
    let gas_coins = conn.keys(format!("{GAS_KEY_PREFIX}*")).await?;

    Ok(gas_coins)
  }

  // It will find the smallest coins that has just enough balance to pay the rebalance_coin transaction block gas cost
  fn get_gas_payment_coin_index(input_coins: &Vec<Coin>) -> Result<usize> {
    // TODO: We need to calculate the amount of gas cost that will be required to pay for the rebalance_coin
    // transaction block;
    let total_gas_cost = GAS_BUDGET;
    
    // find the smallest big enough coin
    let pos = input_coins.iter()
    .rev()
    .position(|c| c.balance >= total_gas_cost)
    .context("no gas payment coin found")?;

    // Get the original index not the reverse
    Ok(input_coins.len() - 1 - pos)
  }

  /// Fetches all coins that belong to the sponsor. It will return a sorted array of coins according to their balance
  async fn fetch_coins(&self) -> Result<Vec<Coin>> {
    let mut coins = vec![];
    let mut cursor = None;

    loop {
      let response = self.api.coin_read_api().get_coins(
        self.sponsor,
        None,
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

  /// It will add all newly created coin object ids to Redis, as well as, push
  /// to the distrubuted queue to be consumed by the Gas Pool.
  pub async fn process_new_coins(&self, new_coins: Vec<ObjectID>) -> Result<()> {
    let new_coins = new_coins.iter()
    .map(|c| format!("{}{}", GAS_KEY_PREFIX, c.to_hex_uncompressed()))
    .collect::<Vec<_>>();

    let len = new_coins.len();
    let mut conn = self.redis_pool.connection().await?;
    // the value is irrelevant; we just use number 1 as a convention
    conn.mset(new_coins, vec!["1".to_string(); len]).await?;
    
    Ok(())
  }

  fn has_errors(response: &SuiTransactionBlockResponse) -> bool {
    if response.errors.len() > 0 {return true}

    if let Some(effects) = response.effects.as_ref() {
      let SuiTransactionBlockEffects::V1(effects) = effects;
      
      if let SuiExecutionStatus::Failure {..} = effects.status {
        return true
      } 
    }

    false
  }

  /// It will first merge all user coins (except for those that are still in the Gas Pool) into the master coin.
  /// Then it split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins; thus rebalancing
  /// Sponsor's coins and keeping Gas Pool liquid.
  async fn rebalance_coins(
    &self,
    mut input_coins: Vec<Coin>,
    gas_pool_coin_count: usize,
  ) -> Result<Vec<ObjectID>> {
    info!("Rebalancing coins...");

    let gas_price = self.gas_meter.gas_price().await?;
    let mut ptb = ProgrammableTransactionBuilder::new();
    
    // Use the first coin as the master coin
    // The master coin and gas payment cannot be used in the input coins that will be merged so we should
    // remove both from the list
    let master_coin_arg = map_err!(ptb.obj(ObjectArg::ImmOrOwnedObject(input_coins[0].object_ref())))?;
    input_coins.remove(0);

    let gas_payment_index = Self::get_gas_payment_coin_index(&input_coins)?;
    let gas_payment = input_coins[gas_payment_index].object_ref();
    input_coins.remove(gas_payment_index);

    // 1. Merge all these coins into the master coin 
    // If the sponsor has only one coin the input_coins (which exclude the master coin) will be empty and thus
    // we can skip the merge step in this iteration.
    if input_coins.len() > 0 {
      let input_coin_args = input_coins.into_iter()
      .map(|c| ptb.obj(ObjectArg::ImmOrOwnedObject(c.object_ref())).expect("coin object ref"))
      .collect::<Vec<_>>();

      let merge_coin_cmd = Command::MergeCoins(master_coin_arg, input_coin_args);
      ptb.command(merge_coin_cmd);
    }

    // 2. Split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT each having `coin_balance`
    let new_coin_count = self.max_capacity - gas_pool_coin_count;
    let amounts = vec![self.coin_balance; new_coin_count]
    .into_iter()
    .map(|a| ptb.pure(a).expect("pure arg"))
    .collect::<Vec<_>>();

    // We could in theory use one single `Command::SplitCoins`. The issue is that `ptb.transfer_arg` on the
    // newly created coin was failing. Instead we just create multiple individual calls.
    let sui_coin_arg_type = map_err!(TypeTag::from_str("0x2::sui::SUI"))?;
    for amount in amounts {
      let new_coin_result = ptb.programmable_move_call(
        SUI_FRAMEWORK_PACKAGE_ID,
        coin::COIN_MODULE_NAME.to_owned(),
        map_err!(Identifier::from_str("split"))?,
        vec![sui_coin_arg_type.clone()],
        vec![master_coin_arg, amount],
      );
  
      ptb.transfer_arg(self.sponsor, new_coin_result);
    }

    let pt = ptb.finish();
    let tx_data = TransactionData::new_programmable(
      self.sponsor,
      vec![gas_payment],
      pt,
      GAS_BUDGET,
      gas_price,
    );

    self.api
    .read_api()
    .dry_run_transaction_block(tx_data.clone())
    .await?;

    let signature = self.wallet.sign(&tx_data, Intent::sui_transaction())?;
    let response = self.api
    .quorum_driver_api()
    .execute_transaction_block(
      Transaction::from_data(tx_data, Intent::sui_transaction(), vec![signature]).verify()?,
      SuiTransactionBlockResponseOptions::full_content(),
      Some(ExecuteTransactionRequestType::WaitForLocalExecution),
    )
    .await
    .expect("successul rebalancing");

    ensure!(!Self::has_errors(&response), "rebalancing failed");

    let new_objects = get_created_objects(&response);
    info!("Suceccessfully rebalanced. Number of new coins {}", new_objects.len());
  
    Ok(new_objects)
  }

  /// Main execution logic
  pub async fn execute(&mut self, current_coins: Vec<String>) -> Result<()> {
    // 1. Load all coins that belong to the sponsor account
    let coins = self.fetch_coins().await?;
    let non_empty_coins = coins.iter().filter(|c| c.balance > 0).count();
    ensure!(non_empty_coins > 1, "Sponsor MUST have at least two coins");
    
    // 2. Exclude the ones that are currently in the Gas Pool
    let input_coins = coins.into_iter()
    .filter(|coin| {
      let (c, _, _) = coin.object_ref();
      !current_coins.contains(&c.to_hex_literal())
    })
    .collect::<Vec<_>>();
  
    // 3. Rebalance coins
    let gas_pool_coin_count = current_coins.len();
    let new_coins = self.rebalance_coins(input_coins, gas_pool_coin_count).await?;
    
    // 4. TODO: Store the new coins into the pool; one Redis entry for each object id
    self.process_new_coins(new_coins).await?;

    Ok(())
  }

  /// A loop that periodically checks if the number of Gas coins in the pool is lower than our capacity
  pub async fn run(&mut self) -> Result<()> {
    loop {
      let pool_coins = self.get_pool_coins().await?;

      if pool_coins.len() <= self.min_pool_count {
        self.execute(pool_coins).await?;
      }
      
      sleep(Duration::from_secs(100)).await;
    }
  }
}
