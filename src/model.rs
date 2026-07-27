use super::*;

pub(crate) struct Model {
  pub(crate) cache_read_tokens: i64,
  pub(crate) cache_write_tokens: i64,
  pub(crate) cost: Option<f64>,
  pub(crate) input_tokens: i64,
  pub(crate) messages: i64,
  pub(crate) name: String,
  pub(crate) output_tokens: i64,
  pub(crate) reasoning_tokens: i64,
}

impl Model {
  pub(crate) fn estimate(&mut self, models: &Models) {
    if self.cost.is_none() {
      self.cost = models.estimate(self);
    }
  }
}
