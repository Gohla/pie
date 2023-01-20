use std::io;
use std::io::{Stderr, Stdout};
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamp, OutputStamp};
use crate::Task;
use crate::tracker::Tracker;

/// A [`Tracker`] that writes events to a [`std::io::Write`] instance, for example [`std::io::Stdout`].
#[derive(Debug, Clone)]
pub struct WritingTracker<W, T> {
  writer: W,
  indentation: u32,
  _task_phantom: PhantomData<T>,
}

impl<T> Default for WritingTracker<Stdout, T> {
  #[inline]
  fn default() -> Self { Self::new_stdout_writer() }
}

impl<T> Default for WritingTracker<Stderr, T> {
  #[inline]
  fn default() -> Self { Self::new_stderr_writer() }
}

impl<W: io::Write, T> WritingTracker<W, T> {
  #[inline]
  pub fn new(writer: W) -> Self { Self { writer, indentation: 0, _task_phantom: Default::default() } }
}

impl<T> WritingTracker<Stdout, T> {
  #[inline]
  pub fn new_stdout_writer() -> Self { Self::new(io::stdout()) }
}

impl<T> WritingTracker<Stderr, T> {
  #[inline]
  pub fn new_stderr_writer() -> Self { Self::new(io::stderr()) }
}

impl<W: io::Write, T: Task> Tracker<T> for WritingTracker<W, T> {
  #[inline]
  fn require_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn provide_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn require_task(&mut self, _task: &T) {}

  #[inline]
  fn execute_task_start(&mut self, task: &T) {
    self.write(format_args!("→ {:?}", task));
    self.indent();
  }
  #[inline]
  fn execute_task_end(&mut self, _task: &T, output: &T::Output) {
    self.unindent();
    self.write(format_args!("← {:?}", output));
  }
  #[inline]
  fn up_to_date(&mut self, task: &T) {
    self.write(format_args!("✓ {:?}", task))
  }

  #[inline]
  fn require_top_down_initial_start(&mut self, task: &T) {
    self.write(format_args!("Top-down build start: {:?}", task));
    self.indent();
  }
  #[inline]
  fn require_top_down_initial_end(&mut self, _task: &T, output: &T::Output) {
    self.unindent();
    self.write(format_args!("Top-down build end: {:?}", output));
  }
  #[inline]
  fn check_top_down_start(&mut self, task: &T) {
    self.write(format_args!("? {:?}", task));
    self.indent();
  }
  #[inline]
  fn check_require_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_require_file_end(&mut self, dependency: &FileDependency, inconsistent: Option<&FileStamp>) {
    self.write_file_dependency(dependency, inconsistent);
  }
  #[inline]
  fn check_provide_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_provide_file_end(&mut self, dependency: &FileDependency, inconsistent: Option<&FileStamp>) {
    self.write_file_dependency(dependency, inconsistent);
  }
  #[inline]
  fn check_require_task_start(&mut self, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<&OutputStamp<T::Output>>) {
    self.write_task_dependency(dependency, inconsistent);
  }
  #[inline]
  fn check_top_down_end(&mut self, _task: &T) {
    self.unindent();
  }

  #[inline]
  fn require_bottom_up_initial_start(&mut self, changed_files: &[PathBuf]) {
    self.write(format_args!("Bottom-up build start: {:?}", changed_files));
    self.indent();
  }
  #[inline]
  fn require_bottom_up_initial_end(&mut self) {
    self.unindent();
    self.write(format_args!("Bottom-up build end"));
  }
  #[inline]
  fn schedule_affected_by_file_start(&mut self, file: &PathBuf) {
    self.write(format_args!("¿ {}", file.display()));
    self.indent();
  }
  #[inline]
  fn check_affected_by_require_file(&mut self, dependency: &FileDependency, inconsistent: Option<&FileStamp>) {
    self.write_file_dependency(dependency, inconsistent);
  }
  #[inline]
  fn check_affected_by_provide_file(&mut self, dependency: &FileDependency, inconsistent: Option<&FileStamp>) {
    self.write_file_dependency(dependency, inconsistent);
  }
  #[inline]
  fn schedule_affected_by_file_end(&mut self, _file: &PathBuf) {
    self.unindent();
  }
  #[inline]
  fn check_affected_by_task_output_start(&mut self, output: &T::Output) {
    self.write(format_args!("¿ {:?}", output));
    self.indent();
  }
  #[inline]
  fn check_affected_by_require_task(&mut self, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<&OutputStamp<T::Output>>) {
    self.write_task_dependency(dependency, inconsistent);
  }
  #[inline]
  fn check_affected_by_task_output_end(&mut self, _output: &T::Output) {
    self.unindent();
  }
  #[inline]
  fn schedule_task(&mut self, task: &T) {
    self.write(format_args!("↑ {:?}", task));
  }
}

impl<W: io::Write, T: Task> WritingTracker<W, T> {
  #[inline]
  fn write(&mut self, args: std::fmt::Arguments) {
    self.write_indentation();
    writeln!(&mut self.writer, "{}", args).ok();
  }
  #[inline]
  fn write_indentation(&mut self) {
    for _ in 0..self.indentation {
      write!(&mut self.writer, " ").ok();
    }
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
  fn write_file_dependency(&mut self, dependency: &FileDependency, inconsistent: Option<&FileStamp>) {
    if let Some(new_stamp) = inconsistent {
      self.write(format_args!("☒ {} ({:?} ≠ {:?})", dependency.path.display(), dependency.stamp, new_stamp));
    } else {
      self.write(format_args!("☑ {} ({:?})", dependency.path.display(), dependency.stamp));
    }
  }
  #[inline]
  fn write_task_dependency(&mut self, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<&OutputStamp<T::Output>>) {
    if let Some(new_stamp) = inconsistent {
      self.write(format_args!("☒ {:?} ({:?} ≠ {:?})", dependency.task, dependency.stamp, new_stamp));
    } else {
      self.write(format_args!("☑ {:?} ({:?})", dependency.task, dependency.stamp));
    }
  }
}
