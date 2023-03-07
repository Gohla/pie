use std::error::Error;
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::dependency::{Dependency, FileDependency, InconsistentDependency, TaskDependency};
use crate::stamp::{FileStamp, OutputStamp};
use crate::Task;

pub mod writing;
pub mod metrics;
pub mod event;

/// Trait for tracking build events. Can be used to implement logging, event tracing, and possibly progress tracking.
pub trait Tracker<T: Task> {
  fn require_file(&mut self, dependency: &FileDependency);
  fn provide_file(&mut self, dependency: &FileDependency);
  fn require_task(&mut self, task: &T);

  fn execute_task_start(&mut self, task: &T);
  fn execute_task_end(&mut self, task: &T, output: &T::Output);
  fn up_to_date(&mut self, task: &T);

  fn require_top_down_initial_start(&mut self, task: &T);
  fn check_top_down_start(&mut self, task: &T);
  fn check_dependency_start(&mut self, dependency: &Dependency<T, T::Output>) {
    match dependency {
      Dependency::RequireFile(d) => self.check_require_file_start(d),
      Dependency::ProvideFile(d) => self.check_provide_file_start(d),
      Dependency::RequireTask(d) => self.check_require_task_start(d),
    }
  }
  fn check_dependency_end(&mut self, dependency: &Dependency<T, T::Output>, inconsistent: Result<Option<&InconsistentDependency<T::Output>>, &dyn Error>) {
    use Dependency::*;
    match dependency {
      RequireFile(d) => {
        let inconsistent = inconsistent.map(|r| r.map(|i| i.unwrap_as_file_stamp()));
        self.check_require_file_end(d, inconsistent);
      }
      ProvideFile(d) => {
        let inconsistent = inconsistent.map(|r| r.map(|i| i.unwrap_as_file_stamp()));
        self.check_provide_file_end(d, inconsistent);
      }
      RequireTask(d) => {
        let inconsistent = inconsistent.unwrap().map(|i| i.unwrap_as_output_stamp());
        self.check_require_task_end(d, inconsistent);
      }
    }
  }
  fn check_require_file_start(&mut self, dependency: &FileDependency);
  fn check_require_file_end(&mut self, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>);
  fn check_provide_file_start(&mut self, dependency: &FileDependency);
  fn check_provide_file_end(&mut self, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>);
  fn check_require_task_start(&mut self, dependency: &TaskDependency<T, T::Output>);
  fn check_require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<&OutputStamp<T::Output>>);
  fn check_top_down_end(&mut self, task: &T);
  fn require_top_down_initial_end(&mut self, task: &T, output: &T::Output);

  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I);
  fn schedule_affected_by_file_start(&mut self, path: &PathBuf);
  fn check_affected_by_file_start(&mut self, requiring_task: &T, dependency: &FileDependency);
  fn check_affected_by_file_end(&mut self, requiring_task: &T, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>);
  fn schedule_affected_by_file_end(&mut self, path: &PathBuf);
  fn schedule_affected_by_task_start(&mut self, task: &T);
  fn check_affected_by_required_task_start(&mut self, requiring_task: &T, dependency: &TaskDependency<T, T::Output>);
  fn check_affected_by_required_task_end(&mut self, requiring_task: &T, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<OutputStamp<&T::Output>>);
  fn schedule_affected_by_task_end(&mut self, task: &T);
  fn schedule_task(&mut self, task: &T);
  fn update_affected_by_end(&mut self);
}


/// A [`Tracker`] that does nothing.
#[derive(Clone, Debug)]
pub struct NoopTracker<T>(PhantomData<T>);

impl<T: Task> Default for NoopTracker<T> {
  #[inline]
  fn default() -> Self { Self(PhantomData::default()) }
}

impl<T: Task> Tracker<T> for NoopTracker<T> {
  #[inline]
  fn require_file(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn provide_file(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn require_task(&mut self, _task: &T) {}

  #[inline]
  fn execute_task_start(&mut self, _task: &T) {}
  #[inline]
  fn execute_task_end(&mut self, _task: &T, _output: &T::Output) {}
  #[inline]
  fn up_to_date(&mut self, _task: &T) {}

  #[inline]
  fn require_top_down_initial_start(&mut self, _task: &T) {}
  #[inline]
  fn check_top_down_start(&mut self, _task: &T) {}
  #[inline]
  fn check_require_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_require_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
  #[inline]
  fn check_provide_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_provide_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
  #[inline]
  fn check_require_task_start(&mut self, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<&OutputStamp<T::Output>>) {}
  #[inline]
  fn check_top_down_end(&mut self, _task: &T) {}
  #[inline]
  fn require_top_down_initial_end(&mut self, _task: &T, _output: &T::Output) {}

  #[inline]
  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf>>(&mut self, _changed_files: I) {}
  #[inline]
  fn schedule_affected_by_file_start(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_file_start(&mut self, _requiring_task: &T, _dependency: &FileDependency) {}
  #[inline]
  fn check_affected_by_file_end(&mut self, _requiring_task: &T, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
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
  fn update_affected_by_end(&mut self) {}
}


/// A [`Tracker`] that forwards events to two other [`Tracker`]s.
#[derive(Default, Clone, Debug)]
pub struct CompositeTracker<A1, A2>(pub A1, pub A2);

impl<T: Task, T1: Tracker<T>, T2: Tracker<T>> Tracker<T> for CompositeTracker<T1, T2> {
  #[inline]
  fn require_file(&mut self, dependency: &FileDependency) {
    self.0.require_file(dependency);
    self.1.require_file(dependency);
  }
  #[inline]
  fn provide_file(&mut self, dependency: &FileDependency) {
    self.0.provide_file(dependency);
    self.1.provide_file(dependency);
  }
  #[inline]
  fn require_task(&mut self, task: &T) {
    self.0.require_task(task);
    self.1.require_task(task);
  }

  #[inline]
  fn execute_task_start(&mut self, task: &T) {
    self.0.execute_task_start(task);
    self.1.execute_task_start(task);
  }
  #[inline]
  fn execute_task_end(&mut self, task: &T, output: &T::Output) {
    self.0.execute_task_end(task, output);
    self.1.execute_task_end(task, output);
  }
  #[inline]
  fn up_to_date(&mut self, task: &T) {
    self.0.up_to_date(task);
    self.1.up_to_date(task);
  }

  #[inline]
  fn require_top_down_initial_start(&mut self, task: &T) {
    self.0.require_top_down_initial_start(task);
    self.1.require_top_down_initial_start(task);
  }
  #[inline]
  fn check_top_down_start(&mut self, task: &T) {
    self.0.check_top_down_start(task);
    self.1.check_top_down_start(task);
  }
  #[inline]
  fn check_require_file_start(&mut self, dependency: &FileDependency) {
    self.0.check_require_file_start(dependency);
    self.1.check_require_file_start(dependency);
  }
  #[inline]
  fn check_require_file_end(&mut self, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>) {
    self.0.check_require_file_end(dependency, inconsistent);
    self.1.check_require_file_end(dependency, inconsistent);
  }
  #[inline]
  fn check_provide_file_start(&mut self, dependency: &FileDependency) {
    self.0.check_provide_file_start(dependency);
    self.1.check_provide_file_start(dependency);
  }
  #[inline]
  fn check_provide_file_end(&mut self, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>) {
    self.0.check_provide_file_end(dependency, inconsistent);
    self.1.check_provide_file_end(dependency, inconsistent);
  }
  #[inline]
  fn check_require_task_start(&mut self, dependency: &TaskDependency<T, T::Output>) {
    self.0.check_require_task_start(dependency);
    self.1.check_require_task_start(dependency);
  }
  #[inline]
  fn check_require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<&OutputStamp<T::Output>>) {
    self.0.check_require_task_end(dependency, inconsistent);
    self.1.check_require_task_end(dependency, inconsistent);
  }
  #[inline]
  fn check_top_down_end(&mut self, task: &T) {
    self.0.check_top_down_end(task);
    self.1.check_top_down_end(task);
  }
  #[inline]
  fn require_top_down_initial_end(&mut self, task: &T, output: &T::Output) {
    self.0.require_top_down_initial_end(task, output);
    self.1.require_top_down_initial_end(task, output);
  }

  #[inline]
  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I) {
    self.0.update_affected_by_start(changed_files.clone());
    self.1.update_affected_by_start(changed_files);
  }
  #[inline]
  fn schedule_affected_by_file_start(&mut self, file: &PathBuf) {
    self.0.schedule_affected_by_file_start(file);
    self.1.schedule_affected_by_file_start(file);
  }
  #[inline]
  fn check_affected_by_file_start(&mut self, requiring_task: &T, dependency: &FileDependency) {
    self.0.check_affected_by_file_start(requiring_task, dependency);
    self.1.check_affected_by_file_start(requiring_task, dependency);
  }
  #[inline]
  fn check_affected_by_file_end(&mut self, requiring_task: &T, dependency: &FileDependency, inconsistent: Result<Option<&FileStamp>, &dyn Error>) {
    self.0.check_affected_by_file_end(requiring_task, dependency, inconsistent);
    self.1.check_affected_by_file_end(requiring_task, dependency, inconsistent);
  }
  #[inline]
  fn schedule_affected_by_file_end(&mut self, file: &PathBuf) {
    self.0.schedule_affected_by_file_end(file);
    self.1.schedule_affected_by_file_end(file);
  }
  #[inline]
  fn schedule_affected_by_task_start(&mut self, task: &T) {
    self.0.schedule_affected_by_task_start(task);
    self.1.schedule_affected_by_task_start(task);
  }
  #[inline]
  fn check_affected_by_required_task_start(&mut self, requiring_task: &T, dependency: &TaskDependency<T, T::Output>) {
    self.0.check_affected_by_required_task_start(requiring_task, dependency);
    self.1.check_affected_by_required_task_start(requiring_task, dependency);
  }
  #[inline]
  fn check_affected_by_required_task_end(&mut self, requiring_task: &T, dependency: &TaskDependency<T, T::Output>, inconsistent: Option<OutputStamp<&T::Output>>) {
    self.0.check_affected_by_required_task_end(requiring_task, dependency, inconsistent);
    self.1.check_affected_by_required_task_end(requiring_task, dependency, inconsistent);
  }
  #[inline]
  fn schedule_affected_by_task_end(&mut self, task: &T) {
    self.0.schedule_affected_by_task_end(task);
    self.1.schedule_affected_by_task_end(task);
  }
  #[inline]
  fn schedule_task(&mut self, task: &T) {
    self.0.schedule_task(task);
    self.1.schedule_task(task);
  }
  #[inline]
  fn update_affected_by_end(&mut self) {
    self.0.update_affected_by_end();
    self.1.update_affected_by_end();
  }
}
