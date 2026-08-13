use super::*;

#[derive(Clone)]
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
        r"
          WITH calls AS (
            SELECT session_id, COUNT(*) AS count
            FROM session_message
            WHERE type = 'assistant'
            GROUP BY session_id
          )
          SELECT
            COALESCE(json_extract(model, '$.providerID') || '/' || json_extract(model, '$.id'), 'unknown'),
            COALESCE(SUM(calls.count), 0),
            COALESCE(SUM(cost), 0.0),
            COALESCE(SUM(tokens_input), 0),
            COALESCE(SUM(tokens_output), 0),
            COALESCE(SUM(tokens_reasoning), 0),
            COALESCE(SUM(tokens_cache_read), 0),
            COALESCE(SUM(tokens_cache_write), 0)
          FROM session_v2
          LEFT JOIN calls ON calls.session_id = session_v2.id
          WHERE (?1 IS NULL OR time_updated >= ?1)
            AND (?2 IS NULL OR directory = ?2)
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
          FROM session_v2
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn aggregates_filtered_v2_usage() {
    let directory = tempfile::tempdir().unwrap();

    let database = directory.path().join("opencode.db");

    let connection = Connection::open(&database).unwrap();

    connection
      .execute_batch(
        r#"
          CREATE TABLE session_v2 (
            id TEXT PRIMARY KEY,
            directory TEXT NOT NULL,
            time_updated INTEGER NOT NULL,
            model TEXT,
            cost REAL NOT NULL,
            tokens_input INTEGER NOT NULL,
            tokens_output INTEGER NOT NULL,
            tokens_reasoning INTEGER NOT NULL,
            tokens_cache_read INTEGER NOT NULL,
            tokens_cache_write INTEGER NOT NULL
          );

          INSERT INTO session_v2 VALUES
            ('included', '/project', 200, '{"providerID":"provider","id":"model"}', 1.5, 10, 20, 30, 40, 50),
            ('old', '/project', 100, '{"providerID":"old","id":"model"}', 2.5, 1, 2, 3, 4, 5),
            ('other', '/other', 200, '{"providerID":"other","id":"model"}', 3.5, 6, 7, 8, 9, 10);

          CREATE TABLE session_message (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            type TEXT NOT NULL
          );

          INSERT INTO session_message VALUES
            ('assistant', 'included', 'assistant'),
            ('user', 'included', 'user'),
            ('old-assistant', 'old', 'assistant'),
            ('other-assistant', 'other', 'assistant');
        "#,
      )
      .unwrap();

    drop(connection);

    let usage = Storage::new(database)
      .usage(&Filter {
        cutoff: Some(150),
        project: Some("/project".into()),
      })
      .unwrap();

    let model = Model {
      cache_read_tokens: 40,
      cache_write_tokens: 50,
      cost: Some(1.5),
      input_tokens: 10,
      messages: 1,
      name: "provider/model".into(),
      output_tokens: 20,
      reasoning_tokens: 30,
    };

    assert_eq!(usage.models, [model]);

    assert_eq!(
      usage,
      Usage {
        cache_read_tokens: 40,
        cache_write_tokens: 50,
        cost: 1.5,
        input_tokens: 10,
        messages: 1,
        models: vec![Model {
          cache_read_tokens: 40,
          cache_write_tokens: 50,
          cost: Some(1.5),
          input_tokens: 10,
          messages: 1,
          name: "provider/model".into(),
          output_tokens: 20,
          reasoning_tokens: 30,
        }],
        output_tokens: 20,
        reasoning_tokens: 30,
        sessions: 1,
      }
    );
  }
}
