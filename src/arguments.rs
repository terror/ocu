use super::*;

#[derive(Parser)]
#[command(about = "A usage monitor for OpenCode")]
pub(crate) struct Arguments {
  #[command(flatten)]
  options: Options,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    let storage = match (&self.options.database, &self.options.data_dir) {
      (Some(database), None) => Storage::new(database.clone()),
      (None, Some(data_dir)) => Storage::new(data_dir.join("opencode.db")),
      (None, None) => Storage::default()?,
      (Some(_), Some(_)) => unreachable!(),
    };

    App::new(storage, self.options)?.run()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn flattens_options() {
    let arguments = Arguments::try_parse_from(["ocu", "--refresh"]).unwrap();

    assert!(arguments.options.refresh);
  }
}
