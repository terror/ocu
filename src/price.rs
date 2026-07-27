use super::*;

#[derive(Clone, Copy)]
pub(crate) struct Price {
  pub(crate) cache_read: f64,
  pub(crate) cache_write: f64,
  pub(crate) input: f64,
  pub(crate) output: f64,
}

impl Price {
  #[allow(clippy::cast_precision_loss)]
  pub(crate) fn estimate(&self, model: &Model) -> f64 {
    (model.input_tokens as f64 * self.input
      + model.output_tokens as f64 * self.output
      + model.reasoning_tokens as f64 * self.output
      + model.cache_read_tokens as f64 * self.cache_read
      + model.cache_write_tokens as f64 * self.cache_write)
      / 1_000_000.0
  }
}
