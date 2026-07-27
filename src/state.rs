use super::*;

pub(crate) struct State {
  pub(crate) should_quit: bool,
  pub(crate) status: Status,
  pub(crate) usage: Usage,
}

impl State {
  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(Action::Quit) => self.should_quit = true,
      Event::Action(Action::Refresh) => return self.refresh(),
      Event::Error(message) => self.status = Status::Failed { message },
      Event::Refresh(Ok(usage)) => {
        self.status = Status::Succeeded { at: Instant::now() };
        self.usage = usage;
      }
      Event::Refresh(Err(error)) => {
        self.status = Status::Failed {
          message: error.to_string(),
        };
      }
      Event::Tick => self.update_status(),
    }

    Vec::new()
  }

  pub(crate) fn new(usage: Usage, status: Status) -> Self {
    Self {
      should_quit: false,
      status,
      usage,
    }
  }

  fn refresh(&mut self) -> Vec<Effect> {
    if matches!(self.status, Status::Loading) {
      return Vec::new();
    }

    self.status = Status::Loading;

    vec![Effect::Refresh]
  }

  fn update_status(&mut self) {
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
    let mut state = State {
      should_quit: false,
      status: Status::Loading,
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

    state.handle_event(Event::Refresh(Err(anyhow::anyhow!("foo"))));

    assert_eq!(state.usage.sessions, 1);

    assert!(matches!(
      state.status,
      Status::Failed { ref message } if message == "foo"
    ));
  }

  #[test]
  fn refresh_action_starts_loading() {
    let mut state = State::new(
      Usage {
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost: 0.0,
        input_tokens: 0,
        messages: 0,
        models: vec![],
        output_tokens: 0,
        reasoning_tokens: 0,
        sessions: 0,
      },
      Status::Idle,
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Refresh)),
      vec![Effect::Refresh]
    );

    assert!(matches!(state.status, Status::Loading));
  }
}
