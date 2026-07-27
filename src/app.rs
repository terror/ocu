use super::*;

pub(crate) struct App {
  options: Options,
  refresh: Option<Receiver<Result<Usage>>>,
  status: Status,
  storage: Storage,
  usage: Usage,
}

impl App {
  fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result {
    loop {
      self.update_status();

      terminal.draw(|frame| self.render(frame))?;

      if event::poll(Duration::from_millis(250))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
      {
        match key.code {
          KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
          KeyCode::Char('r') => self.refresh()?,
          _ => {}
        }
      }
    }
  }

  pub(crate) fn new(storage: Storage, options: Options) -> Result<Self> {
    let mut usage = storage.usage(&Filter::new(&options)?)?;

    let status = match Models::load(options.refresh) {
      Ok(models) => {
        usage.estimate(&models);
        Status::Idle
      }
      Err(error) => Status::Failed {
        message: error.to_string(),
      },
    };

    Ok(Self {
      options,
      refresh: None,
      status,
      storage,
      usage,
    })
  }

  fn refresh(&mut self) -> Result {
    if matches!(self.status, Status::Loading) {
      return Ok(());
    }

    let filter = Filter::new(&self.options)?;
    let storage = self.storage.clone();

    let (sender, receiver) = mpsc::sync_channel(1);

    thread::spawn(move || {
      let result = (|| {
        let mut usage = storage.usage(&filter)?;
        usage.estimate(&Models::load(true).unwrap_or_default());
        Ok(usage)
      })();

      drop(sender.send(result));
    });

    self.refresh = Some(receiver);
    self.status = Status::Loading;

    Ok(())
  }

  fn render(&self, frame: &mut Frame) {
    let area = frame.area();

    let rows = Layout::default()
      .direction(Direction::Vertical)
      .margin(1)
      .constraints([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(10),
        Constraint::Length(1),
        Constraint::Length(1),
      ])
      .split(area);

    frame.render_widget(Paragraph::new("ocu"), rows[0]);

    let cells = [
      format!("{}\nSESSIONS", format_number(self.usage.sessions)),
      format!(
        "{}\nCOST",
        format_cost(Some(self.usage.cost()), self.usage.unpriced())
      ),
      format!("{}\nTOKENS", format_number(self.usage.total_tokens())),
      format!("{}\nASSISTANT MESSAGES", format_number(self.usage.messages)),
    ];

    let columns = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Ratio(1, 4); cells.len()])
      .split(rows[2]);

    for (area, cell) in columns.iter().zip(cells) {
      frame.render_widget(
        Paragraph::new(cell)
          .style(Style::default().fg(Color::Cyan))
          .wrap(Wrap { trim: true }),
        *area,
      );
    }

    let header = Row::new(["MODEL", "CALLS", "COST", "INPUT", "OUTPUT"]).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let models = self.usage.models.iter().map(|model| {
      Row::new([
        Cell::from(model.name.clone()),
        Cell::from(format_number(model.messages)),
        Cell::from(format_cost(model.cost, false)),
        Cell::from(format_number(model.input_tokens)),
        Cell::from(format_number(model.output_tokens)),
      ])
    });

    frame.render_widget(
      Table::new(
        models,
        [
          Constraint::Fill(2),
          Constraint::Length(8),
          Constraint::Length(13),
          Constraint::Length(10),
          Constraint::Length(10),
        ],
      )
      .header(header)
      .column_spacing(1),
      rows[4],
    );

    let (status, color) = match &self.status {
      Status::Failed { message } => (
        format!("Refresh failed: {message} • r retry • q/esc quit"),
        Color::Red,
      ),
      Status::Idle => ("r refresh • q/esc quit".into(), Color::DarkGray),
      Status::Loading => ("Refreshing...".into(), Color::Yellow),
      Status::Succeeded { .. } => ("Refreshed".into(), Color::Green),
    };

    frame.render_widget(
      Paragraph::new(status).style(Style::default().fg(color)),
      rows[6],
    );
  }

  pub(crate) fn run(mut self) -> Result {
    let mut terminal = ratatui::init();
    let result = self.event_loop(&mut terminal);
    ratatui::restore();
    result
  }

  fn update_status(&mut self) {
    let result = self.refresh.as_ref().map(Receiver::try_recv);

    match result {
      Some(Ok(Ok(usage))) => {
        self.usage = usage;
        self.refresh = None;
        self.status = Status::Succeeded { at: Instant::now() };
      }
      Some(Ok(Err(error))) => {
        self.refresh = None;

        self.status = Status::Failed {
          message: error.to_string(),
        };
      }
      Some(Err(TryRecvError::Empty)) | None => {}
      Some(Err(TryRecvError::Disconnected)) => {
        self.refresh = None;

        self.status = Status::Failed {
          message: "could not refresh usage".into(),
        };
      }
    }

    if let Status::Succeeded { at } = &self.status
      && at.elapsed() >= Duration::from_secs(1)
    {
      self.status = Status::Idle;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn failed_refresh_preserves_usage() {
    let (sender, receiver) = mpsc::sync_channel::<Result<Usage>>(1);

    sender.send(Err(anyhow::anyhow!("foo"))).unwrap();

    let mut app = App {
      options: Options {
        data_dir: None,
        database: None,
        days: None,
        project: None,
        refresh: false,
      },
      refresh: Some(receiver),
      status: Status::Loading,
      storage: Storage::new(PathBuf::from("foo")),
      usage: Usage {
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost: 0.0,
        input_tokens: 0,
        messages: 0,
        models: vec![],
        output_tokens: 0,
        reasoning_tokens: 0,
        sessions: 1,
      },
    };

    app.update_status();

    assert_eq!(app.usage.sessions, 1);

    assert!(matches!(
      app.status,
      Status::Failed { ref message } if message == "foo"
    ));

    assert!(app.refresh.is_none());
  }
}
