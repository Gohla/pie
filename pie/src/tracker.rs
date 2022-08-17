use std::io;
use std::io::{Stderr, Stdout};
use std::path::PathBuf;

use crate::DynTask;

pub trait Tracker {
  #[inline]
  fn provide_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn require_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn require_task(&mut self, _task: &dyn DynTask) {}
}


// No-op tracker

#[derive(Default)]
pub struct NoopTracker;

impl Tracker for NoopTracker {}


// Writing tracker

pub struct WritingTracker<W> {
  writer: W,
}

impl Default for WritingTracker<Stdout> {
  #[inline]
  fn default() -> Self { Self::new_stdout_writer() }
}

impl<W: io::Write> WritingTracker<W> {
  #[inline]
  pub fn new(writer: W) -> Self { Self { writer } }
}

impl WritingTracker<Stdout> {
  #[inline]
  pub fn new_stdout_writer() -> Self { Self::new(io::stdout()) }
}

impl WritingTracker<Stderr> {
  #[inline]
  pub fn new_stderr_writer() -> Self { Self::new(io::stderr()) }
}

impl<W: std::io::Write> Tracker for WritingTracker<W> {
  #[inline]
  fn provide_file(&mut self, file: &PathBuf) {
    writeln!(&mut self.writer, "Provided file: {}", file.display()).ok();
  }
  #[inline]
  fn require_file(&mut self, file: &PathBuf) {
    writeln!(&mut self.writer, "Required file: {}", file.display()).ok();
  }
  #[inline]
  fn require_task(&mut self, task: &dyn DynTask) {
    writeln!(&mut self.writer, "Required task: {:?}'", task).ok();
  }
}
