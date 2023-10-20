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

#[allow(dead_code)]
impl<W: Write> WritingTracker<W> {
  fn writeln(&mut self, args: std::fmt::Arguments) {
    self.write_indentation();
    let _ = writeln!(&mut self.writer, "{}", args);
  }
  fn write(&mut self, args: std::fmt::Arguments) {
    let _ = write!(&mut self.writer, "{}", args);
  }
  fn write_nl(&mut self) {
    let _ = write!(&mut self.writer, "\n");
  }

  fn indent(&mut self) {
    self.indentation = self.indentation.saturating_add(1);
  }
  fn unindent(&mut self) {
    self.indentation = self.indentation.saturating_sub(1);
  }
  fn write_indentation(&mut self) {
    for _ in 0..self.indentation {
      let _ = write!(&mut self.writer, " ");
    }
  }

  fn flush(&mut self) {
    let _ = self.writer.flush();
  }
}

impl<W: Write, T: Task> Tracker<T> for WritingTracker<W> {
  fn build_start(&mut self) {
    self.indentation = 0;
  }
  fn build_end(&mut self) {
    self.writeln(format_args!("🏁"));
    self.flush();
  }

  fn require_file_end(&mut self, dependency: &FileDependency) {
    self.writeln(format_args!("r {}", dependency.path().display()));
  }
  fn provide_file_end(&mut self, dependency: &FileDependency) {
    self.writeln(format_args!("p {}", dependency.path().display()));
  }
  fn require_task_start(&mut self, task: &T, _stamper: &OutputStamper) {
    self.writeln(format_args!("→ {:?}", task));
    self.indent();
    self.flush();
  }
  fn require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, output: &T::Output, _was_executed: bool) {
    self.unindent();
    self.writeln(format_args!("← {:?}", output));
    self.flush();
  }

  fn check_dependency_start(&mut self, dependency: &Dependency<T, T::Output>) {
    match dependency {
      Dependency::RequireTask(d) => {
        self.writeln(format_args!("? {:?}", d.task()));
        self.indent();
        self.flush();
      },
      _ => {},
    }
  }
  fn check_dependency_end(
    &mut self,
    dependency: &Dependency<T, T::Output>,
    inconsistency: Result<Option<&Inconsistency<T::Output>>, &io::Error>
  ) {
    match dependency {
      Dependency::RequireFile(d) | Dependency::ProvideFile(d) => {
        match inconsistency {
          Err(e) => self.writeln(format_args!("✗ {} (err: {:?})", d.path().display(), e)),
          Ok(Some(Inconsistency::File(s))) =>
            self.writeln(format_args!("✗ {} (old: {:?} ≠ new: {:?})", d.path().display(), d.stamp(), s)),
          Ok(None) => self.writeln(format_args!("✓ {}", d.path().display())),
          _ => {}, // Other variants cannot occur.
        }
      },
      Dependency::RequireTask(d) => {
        self.unindent();
        match inconsistency {
          Ok(Some(Inconsistency::Task(s))) =>
            self.writeln(format_args!("✗ {:?} (old: {:?} ≠ new: {:?})", d.task(), d.stamp(), s)),
          Ok(None) => self.writeln(format_args!("✓ {:?}", d.task())),
          _ => {}, // Other variants cannot occur.
        }
      }
      Dependency::ReservedRequireTask => {} // Ignore: reserved task dependencies are never checked.
    }
    self.flush()
  }

  fn execute_start(&mut self, task: &T) {
    self.writeln(format_args!("▶ {:?}", task));
    self.indent();
    self.flush();
  }
  fn execute_end(&mut self, _task: &T, output: &T::Output) {
    self.unindent();
    self.writeln(format_args!("◀ {:?}", output));
    self.flush();
  }
}
