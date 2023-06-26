use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use crate::{Context, Session, Task};
use crate::context::SessionExt;
use crate::dependency::{Dependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies recursively in a top-down manner.
pub struct TopDownContext<'p, 's, T, O, A, H> {
  session: &'s mut Session<'p, T, O, A, H>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  #[inline]
  pub fn new(session: &'s mut Session<'p, T, T::Output, A, H>) -> Self {
    Self { session }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    self.session.reset();
    self.session.tracker.require_top_down_initial_start(task);
    let output = self.require_task(task);
    self.session.tracker.require_top_down_initial_end(task, &output);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for TopDownContext<'p, 's, T, T::Output, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.session.tracker.require_task(task);
    let node = self.session.store.get_or_create_task_node(task);

    let dependency = TaskDependency::new_reserved(task.clone(), stamper);
    self.session.reserve_task_require_dependency(&node, task, dependency);

    let output = if !self.session.visited.contains(&node) && self.should_execute_task(&node, task) { // Execute the task, cache and return up-to-date output.
      let previous_executing_task = self.session.pre_execute(node, task);
      let output = task.execute(self);
      self.session.post_execute(previous_executing_task, node, task, output.clone());
      output
    } else { // Return already up-to-date output.
      self.session.tracker.up_to_date(task);
      // No panic: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.session.store.get_task_output(&node).clone();
      output
    };

    self.session.update_reserved_task_require_dependency(&node, output.clone());
    self.session.visited.insert(node);

    output
  }

  #[inline]
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, std::io::Error> {
    self.session.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.session.provide_file_with_stamper(path, stamper)
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { self.session.default_output_stamper() }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { self.session.default_require_file_stamper() }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { self.session.default_provide_file_stamper() }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  /// Checks whether the task should be executed, returning `true` if it should be executed.
  fn should_execute_task(&mut self, node: &TaskNode, task: &T) -> bool {
    self.session.tracker.check_top_down_start(task);

    let mut is_dependency_inconsistent = false;
    // Borrow: because we pass `&mut self` to `is_dependency_inconsistent` for recursive consistency checking, we need
    //         to clone and collect dependencies into a `Vec` here.
    let dependencies: Vec<_> = self.session.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      is_dependency_inconsistent |= self.is_dependency_inconsistent(dependency);
    }
    // Execute if a dependency is inconsistent or if the task has no output (meaning that it has never been executed)
    let should_execute = is_dependency_inconsistent || !self.session.store.task_has_output(node);

    self.session.tracker.check_top_down_end(task);
    should_execute
  }

  #[allow(clippy::wrong_self_convention)]
  #[inline]
  fn is_dependency_inconsistent(&mut self, dependency: Dependency<T, T::Output>) -> bool {
    self.session.tracker.check_dependency_start(&dependency);
    let inconsistent = dependency.is_inconsistent(self);
    self.session.tracker.check_dependency_end(&dependency, inconsistent.as_ref().map(|o| o.as_ref()));
    match inconsistent {
      Ok(Some(_)) => true,
      Err(e) => { // Error while checking: store error and assume inconsistent
        self.session.dependency_check_errors.push(e);
        true
      }
      _ => false,
    }
  }
}
