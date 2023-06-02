use eyre::{Result, Report};
use redlock_async::{Lock};

pub struct RedLock {
  inner: redlock_async::RedLock,
}

impl RedLock {
  pub fn new(redis_hosts: Vec<&str>, password: &str) -> Self {
    let inner = redlock_async::RedLock::new(
      redis_hosts.iter().map(|redis_host| format!("redis://:{}@{}:6379", password, redis_host)).collect()
    );

    Self {inner}
  }

  pub async fn lock(&self, resource: &[u8], ttl: usize) -> Result<Lock> {
    self.inner.lock(resource, ttl)
    .await
    .map_err(|error| Report::msg(format!("{:?}", error)))
  }

  pub async fn unlock(&self, lock: Lock<'_>) {
    self.inner.unlock(&lock).await
  }
}
