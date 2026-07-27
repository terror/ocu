use super::*;

pub(crate) enum Event {
  Action(Action),
  Error(String),
  Refresh(Result<Usage>),
  Tick,
}
