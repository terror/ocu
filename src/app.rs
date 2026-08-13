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

    let header = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Min(10), Constraint::Length(18)])
      .split(rows[0]);

    frame.render_widget(
      Paragraph::new(Line::from(vec![Span::styled(
        "ocu",
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )])),
      header[0],
    );

    let cells = [
      ("SESSIONS", format_number(self.state.usage.sessions)),
      (
        "COST",
        format_cost(Some(self.state.usage.cost()), self.state.usage.unpriced()),
      ),
      ("TOKENS", format_number(self.state.usage.total_tokens())),
      ("MESSAGES", format_number(self.state.usage.messages)),
    ];

    let columns = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Ratio(1, 4); cells.len()])
      .split(rows[2]);

    for (area, cell) in columns.iter().zip(cells) {
      frame.render_widget(
        Paragraph::new(vec![
          Line::from(cell.0).style(Style::default().fg(Color::DarkGray)),
          Line::from(cell.1).style(Style::default().fg(Color::Cyan)),
        ])
        .alignment(Alignment::Center),
        *area,
      );
    }

    let header = Row::new([
      Cell::from("MODEL"),
      Cell::from(Line::from("CALLS").right_aligned()),
      Cell::from(Line::from("COST").right_aligned()),
      Cell::from(Line::from("INPUT").right_aligned()),
      Cell::from(Line::from("OUTPUT").right_aligned()),
    ])
    .style(Style::default().fg(Color::DarkGray))
    .bottom_margin(1);

    let models = self.state.usage.models.iter().map(|model| {
      Row::new([
        Cell::from(model.name.clone())
          .style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(Line::from(format_number(model.messages)).right_aligned()),
        Cell::from(Line::from(format_cost(model.cost, false)).right_aligned())
          .style(Style::default().fg(Color::Cyan)),
        Cell::from(
          Line::from(format_number(model.total_input_tokens())).right_aligned(),
        ),
        Cell::from(
          Line::from(format_number(model.total_output_tokens()))
            .right_aligned(),
        ),
      ])
    });

    frame.render_widget(
      Table::new(
        models,
        [
          Constraint::Fill(2),
          Constraint::Length(8),
          Constraint::Length(13),
          Constraint::Length(15),
          Constraint::Length(15),
        ],
      )
      .header(header)
      .column_spacing(1),
      rows[4],
    );

    let (status, color) = match &self.state.status {
      Status::Failed { message } => {
        (format!("refresh failed: {message}"), Color::Red)
      }
      Status::Idle => ("ready".into(), Color::DarkGray),
      Status::Loading => ("refreshing...".into(), Color::Yellow),
      Status::Succeeded { .. } => ("refreshed".into(), Color::Green),
    };

    let footer = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Min(10), Constraint::Length(20)])
      .split(rows[6]);

    frame.render_widget(
      Paragraph::new(status).style(Style::default().fg(color)),
      footer[0],
    );

    frame.render_widget(
      Paragraph::new("r refresh | q quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Right),
      footer[1],
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
