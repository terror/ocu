use super::*;

pub(crate) struct Storage {
  database: PathBuf,
}

impl Storage {
  pub(crate) fn default() -> Result<Self> {
    let data_home = env::var_os("XDG_DATA_HOME")
      .map(PathBuf::from)
      .or_else(|| {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
      })
      .context(
        "could not determine an OpenCode data directory; pass --database",
      )?;

    Ok(Self::new(data_home.join("opencode").join("opencode.db")))
  }

  fn models(connection: &Connection, filter: &Filter) -> Result<Vec<Model>> {
    let mut statement = connection
      .prepare(
        "
          SELECT
            COALESCE(json_extract(data, '$.providerID') || '/' || json_extract(data, '$.modelID'), 'unknown'),
            COUNT(*),
            COALESCE(SUM(json_extract(data, '$.cost')), 0.0),
            COALESCE(SUM(json_extract(data, '$.tokens.input')), 0),
            COALESCE(SUM(json_extract(data, '$.tokens.output')), 0),
            COALESCE(SUM(json_extract(data, '$.tokens.reasoning')), 0),
            COALESCE(SUM(json_extract(data, '$.tokens.cache.read')), 0),
            COALESCE(SUM(json_extract(data, '$.tokens.cache.write')), 0)
          FROM message
          WHERE json_extract(data, '$.role') = 'assistant'
            AND session_id IN (
              SELECT id
              FROM session
              WHERE (?1 IS NULL OR time_updated >= ?1)
                AND (?2 IS NULL OR directory = ?2)
            )
          GROUP BY 1
          ORDER BY 3 DESC, 2 DESC
        ",
      )
      .context("could not query OpenCode model usage")?;

    statement
      .query_map(params![filter.cutoff, filter.project], |row| {
        let cost = row.get::<_, f64>(2)?;

        Ok(Model {
          cache_read_tokens: row.get(6)?,
          cache_write_tokens: row.get(7)?,
          cost: (cost > 0.0).then_some(cost),
          input_tokens: row.get(3)?,
          messages: row.get(1)?,
          name: row.get(0)?,
          output_tokens: row.get(4)?,
          reasoning_tokens: row.get(5)?,
        })
      })
      .context("could not read OpenCode model usage")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode model usage")
  }

  pub(crate) fn new(database: PathBuf) -> Self {
    Self { database }
  }

  pub(crate) fn usage(&self, filter: &Filter) -> Result<Usage> {
    let connection = Connection::open_with_flags(
      &self.database,
      OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .with_context(|| {
      format!(
        "could not open OpenCode database {}",
        self.database.display()
      )
    })?;

    let (
      sessions,
      cost,
      input_tokens,
      output_tokens,
      reasoning_tokens,
      cache_read_tokens,
      cache_write_tokens,
    ) = connection
      .query_row(
        "
          SELECT
            COUNT(*),
            COALESCE(SUM(cost), 0.0),
            COALESCE(SUM(tokens_input), 0),
            COALESCE(SUM(tokens_output), 0),
            COALESCE(SUM(tokens_reasoning), 0),
            COALESCE(SUM(tokens_cache_read), 0),
            COALESCE(SUM(tokens_cache_write), 0)
          FROM session
          WHERE (?1 IS NULL OR time_updated >= ?1)
            AND (?2 IS NULL OR directory = ?2)
        ",
        params![filter.cutoff, filter.project],
        |row| {
          Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
          ))
        },
      )
      .context("could not summarize OpenCode sessions")?;

    let models = Self::models(&connection, filter)?;

    Ok(Usage {
      cache_read_tokens,
      cache_write_tokens,
      cost,
      input_tokens,
      messages: models.iter().map(|model| model.messages).sum(),
      models,
      output_tokens,
      reasoning_tokens,
      sessions,
    })
  }
}
