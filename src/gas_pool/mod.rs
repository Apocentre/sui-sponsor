pub mod coin_object_producer;

use std::sync::Arc;
use eyre::Result;
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectRef, ObjectID};
use crate::helpers::object::get_object_ref;
use self::coin_object_producer::CoinObjectProducer;

pub struct GasPool {
  api: Arc<SuiClient>,
  coin_object_producer: Arc<CoinObjectProducer>,
}

impl GasPool {
  pub fn new(api: Arc<SuiClient>, coin_object_producer: Arc<CoinObjectProducer>) -> Self {
    Self {api, coin_object_producer}
  }

  /// Core gas pool logic. It will make sure that a safe Gas Coin Object will be used. This means
  /// that we will not risk equiovocation of the Gas objects because a locking mechanism will make sure
  /// that the same Gas Coin will not be used in more than one parallel transactions
  pub async fn gas_object(&self) -> Result<ObjectRef> {
    // TODO: implement the logic of gas pool management to get the correct gas object from the pool
    let object_id = ObjectID::random();

    get_object_ref(Arc::clone(&self.api), object_id).await
  }
}

