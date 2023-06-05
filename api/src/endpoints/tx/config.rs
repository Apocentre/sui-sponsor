use actix_web::{web};
use super::{
  request_gas, transmit_tx
};

// TODO: protect these endpoing using the authn middleware
pub fn config(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::resource("/new").route(web::post().to(request_gas::exec))
  );
  cfg.service(
    web::resource("/submit").route(web::post().to(transmit_tx::exec))
  );
}
