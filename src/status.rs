use super::*;

pub(crate) enum Status {
  Failed { message: String },
  Idle,
  Loading,
  Succeeded { at: Instant },
}
