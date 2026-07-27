use {
  anyhow::Context,
  app::App,
  arguments::Arguments,
  clap::{Args, Parser},
  crossterm::event::{self, Event, KeyCode, KeyEventKind},
  filter::Filter,
  model::Model,
  models::Models,
  options::Options,
  price::Price,
  ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
  },
  rusqlite::{Connection, OpenFlags, params},
  serde::Deserialize,
  status::Status,
  std::{
    cmp::Ordering,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
  },
  storage::Storage,
  usage::Usage,
};

mod app;
mod arguments;
mod filter;
mod model;
mod models;
mod options;
mod price;
mod status;
mod storage;
mod usage;

fn format_cost(cost: Option<f64>, partial: bool) -> String {
  let Some(cost) = cost else {
    return "n/a".into();
  };

  let partial = if partial { "+" } else { "" };

  format!("${cost:.2}{partial}")
}

fn format_number(value: i64) -> String {
  let digits = value.to_string();

  let start = usize::from(digits.starts_with('-'));

  let length = digits.len() - start;

  let mut formatted = digits[..start].to_owned();

  for (index, digit) in digits[start..].chars().enumerate() {
    if index > 0 && (length - index).is_multiple_of(3) {
      formatted.push(',');
    }

    formatted.push(digit);
  }

  formatted
}

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

fn main() {
  if let Err(error) = Arguments::parse().run() {
    eprintln!("error: {error}");

    for cause in error.chain().skip(1) {
      eprintln!("because: {cause}");
    }

    process::exit(1);
  }
}
