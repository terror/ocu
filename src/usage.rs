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
  pub(crate) fn cost(&self) -> f64 {
    if self.models.is_empty() {
      return self.cost;
    }

    self.models.iter().filter_map(|model| model.cost).sum()
  }

  pub(crate) fn estimate(&mut self, models: &Models) {
    for model in &mut self.models {
      model.estimate(models);
    }

    self
      .models
      .sort_by(|left, right| match (left.cost, right.cost) {
        (Some(left_cost), Some(right_cost)) => right_cost
          .total_cmp(&left_cost)
          .then_with(|| left.name.cmp(&right.name)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.name.cmp(&right.name),
      });
  }

  pub(crate) fn total_tokens(&self) -> i64 {
    self.input_tokens
      + self.output_tokens
      + self.reasoning_tokens
      + self.cache_read_tokens
      + self.cache_write_tokens
  }

  pub(crate) fn unpriced(&self) -> bool {
    self.models.iter().any(|model| model.cost.is_none())
  }
}
