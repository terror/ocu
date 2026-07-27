use super::*;

#[derive(Parser)]
#[command(about = "A usage monitor for OpenCode")]
pub(crate) struct Arguments {
  #[arg(
    long,
    value_name = "PATH",
    conflicts_with = "database",
    help = "OpenCode data directory"
  )]
  data_dir: Option<PathBuf>,
  #[arg(long, value_name = "PATH", help = "OpenCode database path")]
  database: Option<PathBuf>,
  #[arg(
    long,
    value_name = "DAYS",
    help = "Only include sessions updated in the last N days"
  )]
  days: Option<u64>,
  #[arg(
    long,
    value_name = "PATH",
    help = "Only include sessions from this project directory"
  )]
  project: Option<PathBuf>,
  #[arg(long, help = "Refresh cached model rates")]
  refresh: bool,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    let storage = match (self.database, self.data_dir) {
      (Some(database), None) => Storage::new(database),
      (None, Some(data_dir)) => Storage::new(data_dir.join("opencode.db")),
      (None, None) => Storage::default()?,
      (Some(_), Some(_)) => unreachable!(),
    };

    App::new(storage, Filter::new(self.days, self.project)?, self.refresh)?
      .run()
  }
}
