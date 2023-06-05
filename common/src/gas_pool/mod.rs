pub mod coin_object_producer;

use std::{sync::Arc, collections::HashMap};
use borsh::BorshDeserialize;
use eyre::{Result, ContextCompat, ensure};
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectRef, ObjectID};
use amqp_helpers::{
  Delivery, consumer::pull_consumer::{PullConsumer, NextItem},
  BasicNackOptions, BasicAckOptions,
};
use crate::{helpers::object::get_object_ref, storage::redis::ConnectionPool};
use self::coin_object_producer::{NewCoinObject};

const GAS_KEY_PREFIX: &str = "gas:";

pub struct GasPool {
  api: Arc<SuiClient>,
  redis_pool: Arc<ConnectionPool>,
  coin_object_consumer: PullConsumer,
  // We need to delivery object to ack/nack messages we receive from RabbitMQ. The process of requesting and confirming
  // gas object is asynchronous. Client first request the GasData object which we get from the queue. Client then will sign
  // a new transaction data including this signed GasData and send it back to us so we can transmit it to the network. It
  // is at this point that we need to put the message back to the queue. However, this whole process requires two HTTP
  // rountrips that happen in an asynchronous way. So we need to store the Delivery instance in memory so we can then
  // identify which message it refers to and act accordingly i.e. put the coin object id back to the queue.
  pending_deliveries: HashMap<String, Delivery>,
}

impl GasPool {
  pub async fn try_new(
    api: Arc<SuiClient>,
    redis_pool: Arc<ConnectionPool>,
    rabbitmq_uri: &str,
  ) -> Self {

    let coin_object_consumer = PullConsumer::new(
      rabbitmq_uri,
      "coin_object",
    ).await.expect("create consumer");

    Self {
      api,
      redis_pool,
      coin_object_consumer,
      pending_deliveries: HashMap::new(),
    }
  }

  /// Returns the given gas coin back to the pool so it can be used in another transaction.
  /// We nack the message so it can be put back to the queue. We use a retry consumer so there is already DLX
  /// and other queue setup that will make sure msg will be put back to the queue after the nack.
  pub async fn return_gas_object(&mut self, coin_object_id: ObjectID) -> Result<()> {
    let delivery = self.pending_deliveries.remove(&coin_object_id.to_hex_uncompressed()).context("coin id not found")?;
    delivery.nack(BasicNackOptions::default()).await?;

    Ok(())
  }

  /// Core gas pool logic. It will make sure that a safe Gas Coin Object will be used. This means
  /// that we will not risk equiovocation of the Gas objects because a locking mechanism will make sure
  /// that the same Gas Coin will not be used in more than one parallel transactions
  pub async fn gas_object(&mut self) -> Result<ObjectRef> {
    let NextItem {
      delivery,
      retry_count: _,
    } = self.coin_object_consumer.next().await?.context("Gas pool empty")?;
    let new_coin_object = NewCoinObject::try_from_slice(&delivery.data).unwrap();

    // This should never happen (in theory) based on the algorithm we use. There can only be one message
    // pointing to the same coin object id in the queue. And since we add the object id back to the pool
    // after we have confirmed the previous transaction is executed, this scenario should be consired
    // impossible
    ensure!(!self.pending_deliveries.contains_key(&new_coin_object.id), "Possible equivocation");

    let coin_object_id = ObjectID::from_hex_literal(&new_coin_object.id)?;
    // Store the msg delivery so we can later nack the message i.e. put it back to the queue.
    self.pending_deliveries.insert(new_coin_object.id, delivery);

    get_object_ref(Arc::clone(&self.api), coin_object_id).await
  }

  /// Removes the given gas coin object from the pool
  pub async fn remove_gas_object(&mut self, coin_object_id: ObjectID) -> Result<()> {
    let coin_object_id_str = coin_object_id.to_hex_uncompressed();
    let mut conn = self.redis_pool.connection().await?;

    // 1. delete from Redis
    conn.delete(format!("{GAS_KEY_PREFIX}{coin_object_id_str}")).await?;

    // 2. remove from RabbitMQ
    let delivery = self.pending_deliveries.remove(&coin_object_id_str).context("coin id not found")?;
    // Ack here has the effect of the message being considered processed and thus removed from the queue
    delivery.ack(BasicAckOptions::default()).await?;

    Ok(())
  }
}

