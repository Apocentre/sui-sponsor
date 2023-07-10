use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use eyre::{eyre, Result};
use sui_types::{transaction::{TransactionKind, GasData}, base_types::SuiAddress};
use crate::utils::error::Error;
use sui_sponsor_common::{
  map_err,
  utils::store::Store,
};

#[derive(Deserialize)]
pub struct Body {
  tx_data: String,
  // TODO: this must be a signature which we can use to recover the sender address
  sender: SuiAddress,
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
  let tx_data: TransactionKind = map_err!(bcs::from_bytes(&tx_data))?;
  let gas_data = store.sponsor.request_gas(tx_data, body.sender).await?;

  Ok(HttpResponse::Ok().json(Response {gas_data}))
}
