use std::sync::Arc;
use eyre::{eyre, Result, ensure};
use shared_crypto::intent::Intent;
use sui_sdk::{SuiClient};
use sui_types::{
  transaction::{GasData, TransactionData, TransactionKind, Command, ProgrammableMoveCall},
  base_types::{ObjectID, SuiAddress}, gas_coin::GasCoin, crypto::Signature,
};
use crate::{gas_pool::GasPool, helpers::object::get_object, map_err};
use super::{
  gas_meter::GasMeter, wallet::Wallet,
};

pub struct Sponsor {
  api: Arc<SuiClient>,
  wallet: Arc<Wallet>,
  gas_meter: Arc<GasMeter>,
  gas_pool: GasPool,
  min_coin_balance: u64,
  max_gas_budget: u64,
}

impl Sponsor {
  pub fn new(
    api: Arc<SuiClient>,
    wallet: Arc<Wallet>,
    gas_meter: Arc<GasMeter>,
    gas_pool: GasPool,
    min_coin_balance: u64,
    max_gas_budget: u64,
  ) -> Self {
    Self {
      api,
      wallet,
      gas_pool,
      gas_meter,
      min_coin_balance,
      max_gas_budget,
    }
  }

  /// TODO: add logic that will check if the given sender address is blacklisted i.e. it caused equivocation
  /// in the past and thus it's not elligible to use the sponsor service anymore
  fn is_blacklisted(_sender: &SuiAddress) -> bool {
    false
  }

  /// TODO: Received a fully qualified function call name (package::module::function) and determine if it's
  /// part of the calls that this sponsor supports.
  fn is_move_call_supported(_fun_call: &str) -> bool {
    true
  }

  /// TODO: makes sure that client does no abuse by trying to execute expensive transactions blocks
  fn is_gas_budget_within_limits(_gas_data: &GasData) -> bool {
    true
  }

  /// Examined the given transaction data and determines if sponsor supports it.
  fn is_tx_supported(tx_data: &TransactionData) -> bool {
    let TransactionData::V1(data) = tx_data;
    if Self::is_blacklisted(&data.sender) {return false};
    let TransactionKind::ProgrammableTransaction(ptx) = &data.kind else {return false};

    // Make sure all commands are supported
    ptx.commands.iter().all(|cmd| match cmd {
      Command::MoveCall(move_call) => {
        let ProgrammableMoveCall {package, module, function, ..} = &**move_call;
        
        if !Self::is_move_call_supported(&format!("{package}::{module}::{function}")) {
          return false
        }

        true
      },
      Command::SplitCoins(_, _)
      | Command::TransferObjects(_, _)
      | Command::MergeCoins(_, _) => return true,
      Command::Publish(_, _)
      | Command::MakeMoveVec(_, _)
      | Command::Upgrade(_, _, _, _) => return false,
    })
  }

  async fn create_gas_data(&mut self) -> Result<GasData> {
    let pubkey = &self.wallet.public();

    let gas_data = GasData {
      payment: vec![self.gas_pool.gas_object().await?],
      owner: pubkey.into(),
      price: self.gas_meter.gas_price().await?,
      budget: self.max_gas_budget,
    };
  
    Ok(gas_data) 
  }

  pub async fn gas_object_processed(&mut self, coin_object_id: ObjectID) -> Result<()> {
    let coin = &get_object(Arc::clone(&self.api), coin_object_id).await?;
    let coin_balance = map_err!(TryInto::<GasCoin>::try_into(coin))?;

    // check if the coin_object_id has enough balance. If not then remove it from the queue i.e. ack
    // as well as, from Redis.
    if coin_balance.value() <= self.min_coin_balance {
      self.gas_pool.remove_gas_object(coin_object_id).await?;
    } else {
      self.gas_pool.return_gas_object(coin_object_id).await?;
    }

    Ok(())
  }

  /// Returns a gas objects for the given transaction data
  pub async fn request_gas(&mut self, tx_data: TransactionData) -> Result<GasData> {
    ensure!(Self::is_tx_supported(&tx_data), "transaction is not supported");
    let gas_data = self.create_gas_data().await?;
    ensure!(Self::is_gas_budget_within_limits(&gas_data), "exceeded gas budget");
    
    Ok(gas_data)
  }

  /// Returns a signature on the entire transaction. This is after the client has requested a gas object
  /// and has signed the given tx_data. After this call, sponsor can transmit the transaction.
  /// Performs the same transaction data checks as in `request_gas`.
  pub async fn sign_tx(&mut self, tx_data: &TransactionData) -> Result<Signature> {
    ensure!(Self::is_tx_supported(&tx_data), "transaction is not supported");
    let TransactionData::V1(tx) = &tx_data;
    ensure!(Self::is_gas_budget_within_limits(&tx.gas_data), "exceeded gas budget");

    self.wallet.sign(&tx_data, Intent::sui_transaction())
  }
}
