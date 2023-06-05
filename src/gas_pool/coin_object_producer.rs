use eyre::Result;
use borsh::{BorshSerialize, BorshDeserialize};
use amqp_helpers::producer::retry_producer::RetryProducer;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct NewCoinObject {
  // The hex value of the coin object id
  id: String,
}

pub struct CoinObjectProducer(RetryProducer);

impl CoinObjectProducer {
  pub async fn new(rabbitmq_uri: String, retry_ttl: u32) -> Self {
    let producer = RetryProducer::new(
      &rabbitmq_uri,
      &"coin_object",
      &"coin_object",
      &"coin_object.new",
      retry_ttl,
      None
    )
    .await
    .unwrap();

    Self(producer)
  }

  pub async fn new_coin_object(&self, id: String) -> Result<()> {
    let msg = NewCoinObject {id};

    self.0
    .publish(&"coin_object", &"coin_object.new", &msg.try_to_vec().unwrap(), true)
    .await
  }
}
