use std::time::Instant;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{eyre, Result};
use sui_types::{transaction::{TransactionData, GasData}};
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
}

pub async fn exec(
  store: web::Data<Store>,
  body: web::Json<Body>,
) -> Result<HttpResponse, Error> {
  let tx_data = map_err!(base64::decode(&body.tx_data))?;
  let tx_data: TransactionData = map_err!(bcs::from_bytes(&tx_data))?;
  
  let start = Instant::now();
  log::info!("Before locking....");
  let gas_data = store.sponsor.request_gas(tx_data).await?;
  let duration = start.elapsed();
  log::info!("Exec time {:?}", duration);
  
  Ok(HttpResponse::Ok().json(Response {gas_data}))
}
