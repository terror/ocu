use super::*;

pub(crate) struct Filter {
  pub(crate) cutoff: Option<i64>,
  pub(crate) project: Option<String>,
}

impl Filter {
  pub(crate) fn new(
    days: Option<u64>,
    project: Option<PathBuf>,
  ) -> Result<Self> {
    let cutoff = days
      .map(|days| {
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .context("could not determine the current time")?;
        let elapsed =
          now.saturating_sub(Duration::from_secs(days.saturating_mul(86_400)));

        i64::try_from(elapsed.as_millis())
          .context("current time is outside OpenCode's timestamp range")
      })
      .transpose()?;

    Ok(Self {
      cutoff,
      project: project.map(|path| path.to_string_lossy().into_owned()),
    })
  }
}
