pub mod coin_object_producer;

use std::{
  sync::Arc, time::{Duration, SystemTime},
};
use tokio::{self, time};
use dashmap::DashMap;
use borsh::BorshDeserialize;
use log::info;
use eyre::{Result, ContextCompat, ensure};
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectRef, ObjectID};
use amqp_helpers::{
  Delivery, consumer::pull_consumer::{PullConsumer, NextItem},
  BasicNackOptions, BasicAckOptions,
};
use crate::{helpers::object::get_object_ref, storage::redis::ConnectionPool};
use self::coin_object_producer::NewCoinObject;

const GAS_KEY_PREFIX: &str = "gas:";

struct DeliveryInfo {
  delivery: Delivery,
  created_at: SystemTime,
}

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
  pending_deliveries: DashMap<String, DeliveryInfo>,
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
      pending_deliveries: DashMap::new(),
    }
  }

  /// It will run periodically and check there are unacked object ids that has been stalled for
  /// a specific period of time i.e. 1 minute. Such object must return back to the queue so they
  /// can be consumer by other transactions
  pub fn spawn_clean_queue(this: Arc<&'static Self>) {
    let this = Arc::clone(&this);
    
    tokio::spawn(async move {
      let mut interval = time::interval(Duration::from_secs(60));
      
      loop {
        info!("Searching for stalled unacked object ids");
        interval.tick().await;

        for (coin_object_id, delivery_info) in this.pending_deliveries.iter().enumerate() {
          if delivery_info.created_at.elapsed().unwrap().as_secs() > 10 {
            info!("Nacking object id {}", coin_object_id);
            delivery_info.delivery.nack(BasicNackOptions::default()).await.unwrap();
          }
        }
      }
    });
  }

  /// Returns the given gas coin back to the pool so it can be used in another transaction.
  /// We nack the message so it can be put back to the queue. We use a retry consumer so there is already DLX
  /// and other queue setup that will make sure msg will be put back to the queue after the nack.
  pub async fn return_gas_object(&self, coin_object_id: ObjectID) -> Result<()> {
    let (_, delivery_info) = self.pending_deliveries.remove(&coin_object_id.to_hex_uncompressed()).context("coin id not found")?;
    delivery_info.delivery.nack(BasicNackOptions::default()).await?;

    Ok(())
  }

  /// Core gas pool logic. It will make sure that a safe Gas Coin Object will be used. This means
  /// that we will not risk equiovocation of the Gas objects because a locking mechanism will make sure
  /// that the same Gas Coin will not be used in more than one parallel transactions
  pub async fn gas_object(&self) -> Result<ObjectRef> {
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
    self.pending_deliveries.insert(new_coin_object.id, DeliveryInfo {
      delivery,
      created_at: SystemTime::now(),
    });

    get_object_ref(Arc::clone(&self.api), coin_object_id).await
  }

  /// Removes the given gas coin object from the pool
  pub async fn remove_gas_object(&self, coin_object_id: ObjectID) -> Result<()> {
    let coin_object_id_str = coin_object_id.to_hex_uncompressed();
    let mut conn = self.redis_pool.connection().await?;

    // 1. delete from Redis
    conn.delete(format!("{GAS_KEY_PREFIX}{coin_object_id_str}")).await?;

    // 2. remove from RabbitMQ
    let (_, delivery_info) = self.pending_deliveries.remove(&coin_object_id_str).context("coin id not found")?;
    // Ack here has the effect of the message being considered processed and thus removed from the queue
    delivery_info.delivery.ack(BasicAckOptions::default()).await?;

    Ok(())
  }
}

