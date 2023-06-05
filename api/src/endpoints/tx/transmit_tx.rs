use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{eyre, Result, ContextCompat};
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::{transaction::{TransactionData}, crypto::{Signature, ToFromBytes}};
use tokio::sync::Mutex;
use crate::utils::error::Error;
use sui_sponsor_common::{
  utils::store::Store, map_err, helpers::tx::TxManager
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
  signature: String,
  transaction_block_bytes: String,
}

#[derive(Serialize)]
pub struct Response<'a> {
  response: Option<SuiTransactionBlockResponse>,
  errors: Vec<&'a str>,
}

pub async fn exec(
  store: web::Data<Mutex<Store>>,
  body: web::Json<Body>,
) -> Result<HttpResponse, Error> {
  let sig_data = map_err!(base64::decode(&body.signature))?;
  let sig = map_err!(Signature::from_bytes(&sig_data))?;
  let tx_block_bytes = map_err!(base64::decode(&body.transaction_block_bytes))?;
  let tx_data: TransactionData = map_err!(bcs::from_bytes(&tx_block_bytes))?;
  let gas_object_id = TxManager::extract_gas_objects_ids(&tx_data);
  let sponsor_sig = store.lock().await.sponsor.sign_tx(&tx_data).await?;
  let response = store.lock().await.tx_manager.send_tx(tx_data, vec![sig, sponsor_sig]).await?;

  let http_response;

  if TxManager::has_errors(&response) {
    // TODO: get the actual error messages from `has_errors`
    http_response = Response {
      response: None,
      errors: vec!["Tx error"],
    };
  } else {
    http_response = Response {
      response: Some(response),
      errors: vec![],
    };
  }

  // return the Gas Coin used for the payment back to the queue. We get the first gas object because
  // We know that we only use on Gas Coin in GasData
  store.lock().await.sponsor
  .gas_object_processed(*gas_object_id.get(0).context("No Gas coin found")?)
  .await?;

  Ok(HttpResponse::Ok().json(http_response))
}
