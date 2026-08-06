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

  pub(crate) fn total_input_tokens(&self) -> i64 {
    self.input_tokens + self.cache_read_tokens + self.cache_write_tokens
  }

  pub(crate) fn total_output_tokens(&self) -> i64 {
    self.output_tokens + self.reasoning_tokens
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn total_tokens() {
    let model = Model {
      cache_read_tokens: 2,
      cache_write_tokens: 3,
      cost: None,
      input_tokens: 1,
      messages: 0,
      name: String::new(),
      output_tokens: 4,
      reasoning_tokens: 5,
    };

    assert_eq!(model.total_input_tokens(), 6);
    assert_eq!(model.total_output_tokens(), 9);
  }
}
