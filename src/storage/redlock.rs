use eyre::{Result, Report};
use rslock::{LockManager, Lock};

pub struct RedLock {
  inner: LockManager,
}

impl RedLock {
  pub fn new(redis_hosts: Vec<&str>, password: &str) -> Self {
    let inner = LockManager::new(
    redis_hosts.iter().map(|redis_host| format!("redis://:{}@{}:6379", password, redis_host)).collect(),
  );

    Self {inner}
  }

  pub async fn lock<'a>(&'a self, resource: &'a [u8], ttl: usize) -> Result<Lock<'a>> {
    self.inner.lock(resource, ttl)
    .await
    .map_err(|error| Report::msg(format!("{:?}", error)))
  }

  pub async fn unlock(&self, lock: Lock<'_>) {
    self.inner.unlock(&lock).await
  }
}
