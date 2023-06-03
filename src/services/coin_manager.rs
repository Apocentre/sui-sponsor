use std::{sync::Arc};
use eyre::{Result, ensure, eyre, ContextCompat};
use shared_crypto::intent::Intent;
use sui_sdk::{SuiClient, rpc_types::{Coin, SuiTransactionBlockResponseOptions}};
use sui_types::{
  base_types::SuiAddress, transaction::{Command, ObjectArg, TransactionData, Transaction},
  programmable_transaction_builder::ProgrammableTransactionBuilder, quorum_driver_types::ExecuteTransactionRequestType,
};
use log::info;
use tokio::time::{sleep, Duration};
use crate::{
  storage::{redis::ConnectionPool}, map_err,
};
use super::{wallet::Wallet, gas_meter::GasMeter};

const GAS_KEY_PREFIX: &str = "gas:";

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
      max_capacity,
      min_pool_count,
      coin_balance,
      sponsor
    }
  }

  // It will find the smallest coins that has just enough balance to pay the rebalance_coin transaction block gas cost
  fn get_gas_payment_coin_index(input_coins: &Vec<Coin>) -> Result<usize> {
    // TODO: We need to calculate the amount of gas cost that will be required to pay for the rebalance_coin
    // transaction block;
    let total_gas_cost = 5_000_000;
    
    input_coins.iter()
    .position(|c| c.balance >= total_gas_cost)
    .context("no gas payment coin found")
  }

  /// It will first merge all user coins (except for those that are still in the Gas Pool) into the master coin.
  /// Then it split the master coin into MAX_POOL_CAPACITY - CURRENT_POOL_COUNT equal coins; thus rebalancing
  /// Sponsor's coins and keeping Gas Pool liquid.
  async fn rebalance_coins(
    &self,
    mut input_coins: Vec<Coin>,
    gas_pool_coin_count: usize,
  ) -> Result<()> {
    info!("Rebalancing coins...");

    let gas_price = self.gas_meter.gas_price().await?;
    let mut ptb = ProgrammableTransactionBuilder::new();
    
    // Use the first coin as the master coin
    let gas_payment_index = Self::get_gas_payment_coin_index(&input_coins)?;
    let gas_payment = input_coins[gas_payment_index].object_ref();
    let master_coin_obj_ref = input_coins[0].object_ref();
    // The master coin and gas payment cannot be used in the input coins that will be merged
    input_coins.remove(0);
    input_coins.remove(gas_payment_index);

    input_coins.iter().for_each(|c| println!("Input coin >>>>>>>> {c:?}"));
    
    // 1. Merge all these coins into the master coin 
    // If the sponsor has only one coin the input_coins (which exclude the master coin) will be empty and thus
    // we can skip the merge step in this iteration.
    if input_coins.len() > 0 {
      let input_coin_args = input_coins.into_iter()
      .map(|c| ptb.obj(ObjectArg::ImmOrOwnedObject(c.object_ref())).expect("coin object ref"))
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

    info!("Number of new coins {}", amounts.len());


    let split_coin_cmd = Command::SplitCoins(
      map_err!(ptb.obj(ObjectArg::ImmOrOwnedObject(master_coin_obj_ref)))?,
      amounts,
    );
    ptb.command(split_coin_cmd);
    
    let pt = ptb.finish();
    let tx_data = TransactionData::new_programmable(
      self.sponsor,
      vec![gas_payment],
      pt,
      100_000,
      gas_price,
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

    info!("Suceccessfully rebalanced coins");

    Ok(())
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
    self.rebalance_coins(input_coins, gas_pool_coin_count).await?;
    
    // 4. TODO: Store the new coins into the pool; one Redis entry for each object id
    
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
