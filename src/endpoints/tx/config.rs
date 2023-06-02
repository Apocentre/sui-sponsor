use actix_web::{web};
use super::request_gas;

// TODO: protect these endpoing using the authn middleware
pub fn config(cfg: &mut web::ServiceConfig) {
  cfg.service(
    web::resource("/gas").route(web::get().to(request_gas::exec))
  );
}
