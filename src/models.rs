use super::*;

#[derive(Deserialize)]
struct CatalogModel {
  cost: Option<CatalogPrice>,
  id: String,
}

#[derive(Deserialize)]
struct CatalogPrice {
  cache_read: Option<f64>,
  cache_write: Option<f64>,
  input: f64,
  output: f64,
}

#[derive(Deserialize)]
struct CatalogProvider {
  models: HashMap<String, CatalogModel>,
}

#[derive(Default)]
pub(crate) struct Models {
  pricing: HashMap<String, Price>,
}

impl Models {
  const MODELS_DEV_URL: &str = "https://models.dev/api.json";

  pub(crate) fn estimate(&self, model: &Model) -> Option<f64> {
    self
      .pricing
      .get(&model.name)
      .map(|price| price.estimate(model))
  }

  pub(crate) fn fetch() -> Result<Self> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .timeout_global(Some(Duration::from_secs(5)))
      .build()
      .into();

    let body = agent
      .get(Self::MODELS_DEV_URL)
      .call()
      .context("could not fetch current model prices")?
      .body_mut()
      .read_to_string()
      .context("could not read current model prices")?;

    Self::parse(&body)
  }

  fn parse(input: &str) -> Result<Self> {
    let catalog =
      serde_json::from_str::<HashMap<String, CatalogProvider>>(input)
        .context("could not parse current model prices")?;

    let mut pricing = HashMap::new();

    for (provider_id, provider) in catalog {
      for (model_id, model) in provider.models {
        let Some(cost) = model.cost else {
          continue;
        };

        let price = Price {
          cache_read: cost.cache_read.unwrap_or(0.0),
          cache_write: cost.cache_write.unwrap_or(0.0),
          input: cost.input,
          output: cost.output,
        };

        pricing.entry(model.id).or_insert(price);

        pricing
          .entry(format!("{provider_id}/{model_id}"))
          .or_insert(price);
      }
    }

    Ok(Self { pricing })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn estimates_unpriced_models() {
    let models = Models::parse(
      r#"
        {
          "openai": {
            "models": {
              "foo": {
                "id": "foo",
                "cost": {
                  "input": 1,
                  "output": 2,
                  "cache_read": 0.5,
                  "cache_write": 0.75
                }
              }
            }
          }
        }
      "#,
    )
    .unwrap();

    let mut model = Model {
      cache_read_tokens: 1_000_000,
      cache_write_tokens: 1_000_000,
      cost: None,
      input_tokens: 1_000_000,
      messages: 0,
      name: "openai/foo".into(),
      output_tokens: 1_000_000,
      reasoning_tokens: 1_000_000,
    };

    model.estimate(&models);

    assert!((model.cost.unwrap() - 6.25).abs() < f64::EPSILON);
  }
}
