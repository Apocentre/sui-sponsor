// Executed the provided function and converts the Result into eyre::Result
#[macro_export]
macro_rules! map_err {
  ($fun:expr) => {
    $fun.map_err(|e| eyre!(Box::new(e)))
  }
}
