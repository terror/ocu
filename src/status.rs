use super::*;

pub(crate) enum Status {
  Controls,
  Loading,
  Success(Instant),
}
