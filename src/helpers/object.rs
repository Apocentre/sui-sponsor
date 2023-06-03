use std::sync::Arc;
use eyre::Result;
use sui_sdk::{SuiClient, rpc_types::{SuiObjectDataOptions, SuiTransactionBlockResponse, ObjectChange}};
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

pub fn get_created_objects(response: &SuiTransactionBlockResponse) -> Vec<ObjectID> {
  let mut new_objects = vec![];

  if let Some(object_changes) = response.object_changes {
    let objs = object_changes.iter()
    .filter(|obj| if let ObjectChange::Created {object_id, ..} = obj {true} else {false})
    .map(|obj| obj.object_id());

    new_objects.extend(objs)
  };

  new_objects
}
