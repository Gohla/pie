use std::io;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamp, OutputStamp};
use crate::Task;
use crate::tracker::Tracker;

#[derive(Clone, Debug)]
pub struct MetricsTracker<T> {
  report: Report,
  clear_on_build_start: bool,
  last_build_start: Option<Instant>,
  _task_phantom: PhantomData<T>,
}

impl<T> Default for MetricsTracker<T> {
  fn default() -> Self {
    Self {
      report: Report::default(),
      clear_on_build_start: true,
      last_build_start: None,
      _task_phantom: PhantomData::default(),
    }
  }
}

impl<T> MetricsTracker<T> {
  #[inline]
  pub fn report(&self) -> &Report { &self.report }
}

#[derive(Default, Clone, Debug)]
pub struct Report {
  pub total_required_files: u32,
  pub total_provided_files: u32,
  pub total_required_tasks: u32,

  pub total_executed_tasks: u32,
  pub total_required_tasks_up_to_date: u32,

  pub build_duration: Duration,
}

impl Report {
  fn clear(&mut self) {
    self.total_required_files = 0;
    self.total_provided_files = 0;
    self.total_required_tasks = 0;

    self.total_executed_tasks = 0;
    self.total_required_tasks_up_to_date = 0;

    self.build_duration = Duration::default();
  }
}

impl<T: Task> Tracker<T> for MetricsTracker<T> {
  #[inline]
  fn require_file(&mut self, _dependency: &FileDependency) {
    self.report.total_required_files += 1;
  }
  #[inline]
  fn provide_file(&mut self, _dependency: &FileDependency) {
    self.report.total_provided_files += 1;
  }
  #[inline]
  fn require_task_start(&mut self, _task: &T) {
    self.report.total_required_tasks += 1;
  }
  #[inline]
  fn require_task_end(&mut self, _task: &T, _output: &T::Output, was_executed: bool) {
    if !was_executed {
      self.report.total_required_tasks_up_to_date += 1;
    }
  }

  #[inline]
  fn execute_task_start(&mut self, _task: &T) {
    self.report.total_executed_tasks += 1;
  }
  #[inline]
  fn execute_task_end(&mut self, _task: &T, _output: &T::Output) {}

  #[inline]
  fn require_top_down_initial_start(&mut self, _task: &T) {
    if self.clear_on_build_start {
      self.report.clear();
    }
    self.last_build_start = Some(Instant::now());
  }
  #[inline]
  fn check_top_down_start(&mut self, _task: &T) {}
  #[inline]
  fn check_require_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_require_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn check_provide_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_provide_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn check_require_task_start(&mut self, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<&OutputStamp<T::Output>>) {}
  #[inline]
  fn check_top_down_end(&mut self, _task: &T) {}
  #[inline]
  fn require_top_down_initial_end(&mut self, _task: &T, _output: &T::Output) {
    if let Some(start) = &self.last_build_start {
      self.report.build_duration = start.elapsed();
    }
  }

  #[inline]
  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf>>(&mut self, _changed_files: I) {
    if self.clear_on_build_start {
      self.report.clear();
    }
    self.last_build_start = Some(Instant::now());
  }
  #[inline]
  fn schedule_affected_by_file_start(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_file_start(&mut self, _requiring_task: &T, _dependency: &FileDependency) {}
  #[inline]
  fn check_affected_by_file_end(&mut self, _requiring_task: &T, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn schedule_affected_by_file_end(&mut self, _file: &PathBuf) {}
  #[inline]
  fn schedule_affected_by_task_start(&mut self, _task: &T) {}
  #[inline]
  fn check_affected_by_required_task_start(&mut self, _requiring_task: &T, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_affected_by_required_task_end(&mut self, _requiring_task: &T, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<OutputStamp<&T::Output>>) {}
  #[inline]
  fn schedule_affected_by_task_end(&mut self, _task: &T) {}
  #[inline]
  fn schedule_task(&mut self, _task: &T) {}
  #[inline]
  fn update_affected_by_end(&mut self) {
    if let Some(start) = &self.last_build_start {
      self.report.build_duration = start.elapsed();
    }
  }
}
