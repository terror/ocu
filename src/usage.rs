use super::*;

pub(crate) struct Usage {
  pub(crate) cache_read_tokens: i64,
  pub(crate) cache_write_tokens: i64,
  pub(crate) cost: f64,
  pub(crate) input_tokens: i64,
  pub(crate) messages: i64,
  pub(crate) models: Vec<Model>,
  pub(crate) output_tokens: i64,
  pub(crate) reasoning_tokens: i64,
  pub(crate) sessions: i64,
}

impl Usage {
  pub(crate) fn total_tokens(&self) -> i64 {
    self.input_tokens
      + self.output_tokens
      + self.reasoning_tokens
      + self.cache_read_tokens
      + self.cache_write_tokens
  }
}
