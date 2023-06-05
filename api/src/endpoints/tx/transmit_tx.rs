use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{eyre, Result};
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
pub struct Response;

pub async fn exec(
  store: web::Data<Mutex<Store>>,
  body: web::Json<Body>,
) -> Result<HttpResponse, Error> {
  let sig_data = map_err!(base64::decode(&body.signature))?;
  let sig = map_err!(Signature::from_bytes(&sig_data))?;
  let tx_block_bytes = map_err!(base64::decode(&body.transaction_block_bytes))?;
  let tx_data: TransactionData = map_err!(bcs::from_bytes(&tx_block_bytes))?;
  let sponsor_sig = store.lock().await.sponsor.sign_tx(&tx_data).await?;

  let response = store.lock().await.tx_manager.send_tx(tx_data, vec![sig, sponsor_sig]).await?;

  if TxManager::has_errors(&response) {
    return Err(eyre!("Tx error"))?
  }

  Ok(HttpResponse::Ok().json(Response {}))
}
