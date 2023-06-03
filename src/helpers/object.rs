use std::sync::Arc;
use eyre::Result;
use sui_sdk::{SuiClient, rpc_types::SuiObjectDataOptions};
use sui_types::base_types::{ObjectID, ObjectRef};

pub async fn get_object_ref(api: Arc<SuiClient>, object_id: ObjectID) -> Result<ObjectRef> {
  let object = api.read_api().get_object_with_options(
    object_id,
    SuiObjectDataOptions::new().with_type()
  )
  .await?
  .into_object()?;

  Ok(object.object_ref())
}
