use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{Result};
use sui_types::{transaction::{TransactionData, GasData}, crypto::Signature};
use tokio::sync::Mutex;
use crate::utils::error::Error;
use sui_sponsor_common::{
  utils::store::Store,
};

#[derive(Deserialize)]
pub struct Body {
  tx_data: TransactionData,
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
  let (gas_data, sig) = store.lock().await.sponsor.request_gas(body.tx_data.clone()).await?;
  Ok(HttpResponse::Ok().json(Response {gas_data, sig}))
}
