use std::sync::Arc;
use eyre::Result;
use sui_sdk::{SuiClient, rpc_types::SuiObjectDataOptions};
use sui_types::base_types::{ObjectRef, ObjectID};

pub struct GasPool {
  api: Arc<SuiClient>
}

impl GasPool {
  pub fn new(api: Arc<SuiClient>) -> Self {
    Self {api}
  }

  /// Core gas pool logic. It will make sure that a safe Gas Coin Object will be used. This means
  /// that we will not risk equiovocation of the Gas objects because a locking mechanism will make sure
  /// that the same Gas Coin will not be used in more than one parallel transactions
  pub async fn gas_object(&self) -> Result<ObjectRef> {
    // TODO: implement the logic of gas pool management to get the correct gas object from the pool
    let object_id = ObjectID::random();

    let object = self.api.read_api().get_object_with_options(
      object_id,
      SuiObjectDataOptions::new().with_type()
    )
    .await?
    .into_object()?;

    Ok(object.object_ref())
  }
}

