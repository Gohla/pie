use std::error::Error;
use std::fmt::Debug;
use std::io::{self, BufWriter, Stderr, Stdout, Write};

use crate::tracker::Tracker;
use crate::trait_object::{KeyObj, ValueObj};

/// A [`Tracker`] that writes events to a [`Write`] instance, for example [`Stdout`].
#[derive(Clone, Debug)]
pub struct WritingTracker<W> {
  writer: W,
  indentation: u32,
}

impl WritingTracker<BufWriter<Stdout>> {
  /// Creates a [`WritingTracker`] that writes to buffered standard output.
  #[inline]
  pub fn with_stdout() -> Self { Self::new(BufWriter::new(io::stdout())) }
}
impl WritingTracker<BufWriter<Stderr>> {
  /// Creates a [`WritingTracker`] that writes to buffered standard error.
  #[inline]
  pub fn with_stderr() -> Self { Self::new(BufWriter::new(io::stderr())) }
}
impl<W: Write> WritingTracker<W> {
  /// Creates a new [`WritingTracker`] that writes to `writer`.
  #[inline]
  pub fn new(writer: W) -> Self {
    Self {
      writer,
      indentation: 0,
    }
  }
}

#[allow(dead_code)]
impl<W: Write> WritingTracker<W> {
  #[inline]
  fn writeln(&mut self, args: std::fmt::Arguments) {
    self.write_indentation();
    let _ = writeln!(&mut self.writer, "{}", args);
  }
  #[inline]
  fn write(&mut self, args: std::fmt::Arguments) {
    let _ = write!(&mut self.writer, "{}", args);
  }
  #[inline]
  fn write_nl(&mut self) {
    let _ = write!(&mut self.writer, "\n");
  }

  #[inline]
  fn indent(&mut self) {
    self.indentation = self.indentation.saturating_add(1);
  }
  #[inline]
  fn unindent(&mut self) {
    self.indentation = self.indentation.saturating_sub(1);
  }
  #[inline]
  fn write_indentation(&mut self) {
    for _ in 0..self.indentation {
      let _ = write!(&mut self.writer, " ");
    }
  }

  #[inline]
  fn flush(&mut self) {
    let _ = self.writer.flush();
  }
}

impl<W: Write + 'static> Tracker for WritingTracker<W> {
  #[inline]
  fn build_start(&mut self) {
    self.indentation = 0;
  }
  #[inline]
  fn build_end(&mut self) {
    self.writeln(format_args!("🏁"));
    self.flush();
  }

  #[inline]
  fn require_start(&mut self, task: &dyn KeyObj, _checker: &dyn ValueObj) {
    self.writeln(format_args!("→ {:?}", task));
    self.indent();
    self.flush();
  }
  #[inline]
  fn require_end(
    &mut self,
    _task: &dyn KeyObj,
    _checker: &dyn ValueObj,
    _stamp: &dyn ValueObj,
    output: &dyn ValueObj,
  ) {
    self.unindent();
    self.writeln(format_args!("← {:?}", output));
    self.flush();
  }

  #[inline]
  fn read_end(&mut self, resource: &dyn KeyObj, _checker: &dyn ValueObj, _stamp: &dyn ValueObj) {
    self.writeln(format_args!("r {:?}", resource)); // TODO: expose and use display?
  }
  #[inline]
  fn write_end(&mut self, resource: &dyn KeyObj, _checker: &dyn ValueObj, _stamp: &dyn ValueObj) {
    self.writeln(format_args!("w {:?}", resource)); // TODO: expose and use display?
  }

  #[inline]
  fn check_task_start(&mut self, task: &dyn KeyObj, _checker: &dyn ValueObj, _stamp: &dyn ValueObj) {
    self.writeln(format_args!("? {:?}", task));
    self.indent();
    self.flush();
  }
  #[inline]
  fn check_task_end(
    &mut self,
    task: &dyn KeyObj,
    _checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Option<&dyn Debug>,
  ) {
    self.unindent();
    if let Some(new_stamp) = inconsistency {
      self.writeln(format_args!("✗ {:?} (new: {:?} ≉ old: {:?})", task, new_stamp, stamp))
    } else {
      self.writeln(format_args!("✓ {:?}", task))
    }
    self.flush();
  }
  #[inline]
  fn check_resource_end(
    &mut self,
    resource: &dyn KeyObj,
    _checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Result<Option<&dyn Debug>, &dyn Error>,
  ) {
    match inconsistency { // TODO: expose and use display?
      Err(e) => self.writeln(format_args!("✗ {:?} (err: {:?})", resource, e)),
      Ok(Some(new_stamp)) =>
        self.writeln(format_args!("✗ {:?} (new: {:?} ≉ old: {:?})", resource, new_stamp, stamp)),
      Ok(None) => self.writeln(format_args!("✓ {:?}", resource)),
    }
  }

  #[inline]
  fn execute_start(&mut self, task: &dyn KeyObj) {
    self.writeln(format_args!("▶ {:?}", task));
    self.indent();
    self.flush();
  }
  #[inline]
  fn execute_end(&mut self, _task: &dyn KeyObj, output: &dyn ValueObj) {
    self.unindent();
    self.writeln(format_args!("◀ {:?}", output));
    self.flush();
  }
}
