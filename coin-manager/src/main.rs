
use std::{
  env, panic, process, sync::Arc,
};
use eyre::Result;
use env_logger::Env;
use sui_sponsor_common::utils::store::Store;
use sui_sponsor_coin_manager::coin_manager::CoinManager;

#[tokio::main]
async fn main() -> Result<()> {
  let orig_hook = panic::take_hook();
  panic::set_hook(Box::new(move |panic_info| {
    orig_hook(panic_info);
    process::exit(1);
  }));

  if env::var("ENV").unwrap() == "development" {
    dotenv::from_filename(".env").expect("cannot load env from a file");
  }

  let store = Store::new().await;
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

  let sponsor_address = store.wallet.address();
  let mut coin_manager = CoinManager::new(
    Arc::clone(&store.rpc_client),
    Arc::clone(&store.wallet),
    Arc::clone(&store.gas_meter),
    Arc::clone(&store.redis_pool),
    Arc::clone(&store.coin_object_producer),
    store.config.gas_pool.max_capacity,
    store.config.gas_pool.min_pool_count,
    store.config.gas_pool.coin_balance,
    sponsor_address,
  );

  coin_manager.run().await?;

  Ok(())
}
