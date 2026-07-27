use super::*;

pub(crate) struct App {
  event_receiver: Receiver<Event>,
  event_sender: Sender<Event>,
  options: Options,
  state: State,
  storage: Storage,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(250);

  fn drain_pending_events(&mut self) {
    while let Ok(event) = self.event_receiver.try_recv() {
      self.handle_event(event);
    }
  }

  fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result {
    while !self.state.should_quit {
      terminal.draw(|frame| self.render(frame))?;

      let event = match self.event_receiver.recv_timeout(Self::TICK_INTERVAL) {
        Ok(event) => event,
        Err(RecvTimeoutError::Timeout) => Event::Tick,
        Err(RecvTimeoutError::Disconnected) => return Ok(()),
      };

      self.handle_event(event);
      self.drain_pending_events();
    }

    Ok(())
  }

  fn handle_effect(&mut self, effect: Effect) {
    match effect {
      Effect::Refresh => self.refresh(),
    }
  }

  fn handle_event(&mut self, event: Event) {
    for effect in self.state.handle_event(event) {
      self.handle_effect(effect);
    }
  }

  fn listen_for_input(&self) {
    let sender = self.event_sender.clone();

    thread::spawn(move || {
      loop {
        let event = match crossterm_event::read() {
          Ok(event) => event,
          Err(error) => {
            drop(sender.send(Event::Error(error.to_string())));
            return;
          }
        };

        let crossterm_event::Event::Key(key) = event else {
          continue;
        };

        if key.kind != KeyEventKind::Press {
          continue;
        }

        let action = match key.code {
          KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
          KeyCode::Char('r') => Action::Refresh,
          _ => continue,
        };

        if sender.send(Event::Action(action)).is_err() {
          return;
        }
      }
    });
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

    let (event_sender, event_receiver) = mpsc::channel();

    Ok(Self {
      event_receiver,
      event_sender,
      options,
      state: State::new(usage, status),
      storage,
    })
  }

  fn refresh(&self) {
    let filter = match Filter::new(&self.options) {
      Ok(filter) => filter,
      Err(error) => {
        drop(self.event_sender.send(Event::Refresh(Err(error))));
        return;
      }
    };

    let sender = self.event_sender.clone();

    let storage = self.storage.clone();

    thread::spawn(move || {
      let result = (|| {
        let mut usage = storage.usage(&filter)?;
        usage.estimate(&Models::load(true).unwrap_or_default());
        Ok(usage)
      })();

      drop(sender.send(Event::Refresh(result)));
    });
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
      format!("{}\nSESSIONS", format_number(self.state.usage.sessions)),
      format!(
        "{}\nCOST",
        format_cost(Some(self.state.usage.cost()), self.state.usage.unpriced())
      ),
      format!("{}\nTOKENS", format_number(self.state.usage.total_tokens())),
      format!(
        "{}\nASSISTANT MESSAGES",
        format_number(self.state.usage.messages)
      ),
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

    let models = self.state.usage.models.iter().map(|model| {
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

    let (status, color) = match &self.state.status {
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

    self.listen_for_input();

    let result = self.event_loop(&mut terminal);

    ratatui::restore();

    result
  }
}
