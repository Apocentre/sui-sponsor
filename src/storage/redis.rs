use std::iter::zip;
use deadpool_redis::{Pool, Config, Connection, Runtime};
use redis::cmd;
use eyre::Result;

pub struct ConnectionPool(Pool);

impl ConnectionPool {
  pub fn new(redis_host: &str, password: &str, port: u16) -> Self {
    let conn_string = format!("redis://:{}@{}:{}", password, redis_host, port);
    let config = Config::from_url(conn_string);
    let pool = config.create_pool(Some(Runtime::Tokio1)).unwrap();

    Self(pool)
  }

  pub async fn connection(&self) -> Result<Redis> {
    let conn = self.0.get().await?;
    Ok(Redis::new(conn))
  }
}

pub struct Redis(Connection);

impl Redis {
  fn new(connection: Connection) -> Self {
    Redis(connection)
  }

  pub async fn sett<T: AsRef<str>>(&mut self, key: T, value: T) -> Result<()> {
    cmd("SET")
    .arg(&[key.as_ref(), value.as_ref()])
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn mset<T: AsRef<str>>(&mut self, keys: Vec<T>, values: Vec<T>) -> Result<()> {
    let keys = keys.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let values = values.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let args = zip(keys, values).collect::<Vec<_>>();

    cmd("MSET")
    .arg(&[args])
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn set_ext<T: AsRef<str>>(&mut self, key: T, value: T, secs: usize) -> Result<()> {
    cmd("SETEX")
    .arg(&[key.as_ref(), &secs.to_string(), value.as_ref()])
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn gett<T: AsRef<str>>(&mut self, key: T) -> Result<String> {
    cmd("GET")
    .arg(&[key.as_ref()])
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn mget<T: AsRef<str>>(&mut self, keys: &[T]) -> Result<Vec<String>> {
    let keys = keys.iter().map(AsRef::as_ref).collect::<Vec<_>>();

    cmd("MGET")
    .arg(keys)
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn deletet<T: AsRef<str>>(&mut self, key: T) -> Result<()> {
    cmd("DEL")
    .arg(key.as_ref())
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }

  pub async fn keys<T: AsRef<str>>(&mut self, key_pattern: T) -> Result<Vec<String>> {
    cmd("KEYS")
    .arg(key_pattern.as_ref())
    .query_async(&mut self.0).await
    .map_err(Into::<_>::into)
  }
}
