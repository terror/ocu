use super::*;

#[derive(Args)]
pub(crate) struct Options {
  #[arg(
    long,
    value_name = "PATH",
    conflicts_with = "database",
    help = "OpenCode data directory"
  )]
  pub(crate) data_dir: Option<PathBuf>,
  #[arg(long, value_name = "PATH", help = "OpenCode database path")]
  pub(crate) database: Option<PathBuf>,
  #[arg(
    long,
    value_name = "DAYS",
    help = "Only include sessions updated in the last N days"
  )]
  pub(crate) days: Option<u64>,
  #[arg(
    long,
    value_name = "PATH",
    help = "Only include sessions from this project directory"
  )]
  pub(crate) project: Option<PathBuf>,
  #[arg(long, help = "Refresh cached model rates")]
  pub(crate) refresh: bool,
}
