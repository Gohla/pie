use std::io::{self, BufWriter, Stderr, Stdout, Write};

use crate::dependency::{Dependency, FileDependency, Inconsistency, TaskDependency};
use crate::stamp::OutputStamper;
use crate::Task;
use crate::tracker::Tracker;

/// [`Tracker`] that writes events to a [`Write`] instance, for example [`Stdout`].
#[derive(Clone, Debug)]
pub struct WritingTracker<W> {
  writer: W,
  indentation: u32,
}

impl WritingTracker<BufWriter<Stdout>> {
  /// Creates a [`WritingTracker`] that writes to buffered standard output.
  pub fn with_stdout() -> Self { Self::new(BufWriter::new(io::stdout())) }
}
impl WritingTracker<BufWriter<Stderr>> {
  /// Creates a [`WritingTracker`] that writes to buffered standard error.
  pub fn with_stderr() -> Self { Self::new(BufWriter::new(io::stderr())) }
}
impl<W: Write> WritingTracker<W> {
  /// Creates a [`WritingTracker`] that writes to `writer`.
  pub fn new(writer: W) -> Self {
    Self {
      writer,
      indentation: 0,
    }
  }
}
