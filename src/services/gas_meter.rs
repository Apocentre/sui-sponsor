pub struct GasMeter;

impl GasMeter {
  pub fn new() -> Self {
    Self {}
  }

  pub fn gas_price(&self) -> u64 {
    // TODO: Get the current ga price
    0
  }

  pub fn gas_budget(&self) -> u64 {
    // TODO: Compute gas estimation
    0
  }
}
