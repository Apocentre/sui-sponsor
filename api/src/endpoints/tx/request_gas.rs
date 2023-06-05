use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{eyre, Result};
use sui_types::{transaction::{TransactionData, GasData}, crypto::Signature};
use tokio::sync::Mutex;
use crate::utils::error::Error;
use sui_sponsor_common::{
  map_err,
  utils::store::Store,
};

#[derive(Deserialize)]
pub struct Body {
  tx_data: String,
}

#[derive(Serialize)]
pub struct Response {
  gas_data: GasData,
  sig: Signature,
}

pub async fn exec(
  store: web::Data<Mutex<Store>>,
  body: web::Json<Body>,
) -> Result<HttpResponse, Error> {
  let tx_data = map_err!(base64::decode(&body.tx_data))?;
  let tx_data: TransactionData = map_err!(bcs::from_bytes(&tx_data))?;
  let (gas_data, sig) = store.lock().await.sponsor.request_gas(tx_data).await?;
  Ok(HttpResponse::Ok().json(Response {gas_data, sig}))
}
