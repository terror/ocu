use super::*;

pub(crate) struct App {
  filter: Filter,
  storage: Storage,
  usage: Usage,
}

impl App {
  fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result {
    loop {
      terminal.draw(|frame| self.render(frame))?;

      if event::poll(Duration::from_millis(250))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
      {
        match key.code {
          KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
          KeyCode::Char('r') => {
            self.usage = self.storage.usage(&self.filter)?;
            self.usage.estimate(&Models::fetch().unwrap_or_default());
          }
          _ => {}
        }
      }
    }
  }

  pub(crate) fn new(storage: Storage, filter: Filter) -> Result<Self> {
    let mut usage = storage.usage(&filter)?;

    usage.estimate(&Models::fetch().unwrap_or_default());

    Ok(Self {
      filter,
      storage,
      usage,
    })
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

    frame.render_widget(
      Paragraph::new("r refresh • q/esc quit")
        .style(Style::default().fg(Color::DarkGray)),
      rows[6],
    );
  }

  pub(crate) fn run(mut self) -> Result {
    let mut terminal = ratatui::init();
    let result = self.event_loop(&mut terminal);
    ratatui::restore();
    result
  }
}
