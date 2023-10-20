use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use crate::{Context, Session, Task};
use crate::context::SessionExt;
use crate::dependency::{Dependency, MakeConsistent};
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

  /// Requires `task`, returning its consistent output.
  #[inline]
  pub fn require_initial(&mut self, task: &T) -> T::Output {
    self.session.tracker.require_top_down_initial_start(task);
    let output = self.require_task(task);
    self.session.tracker.require_top_down_initial_end(task, &output);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for TopDownContext<'p, 's, T, T::Output, A, H> {
  #[inline]
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, std::io::Error> {
    self.session.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.session.provide_file_with_stamper(path, stamper)
  }
  #[inline]
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.session.tracker.require_task_start(task);

    let node = self.session.store.get_or_create_task_node(task);
    self.session.reserve_task_require_dependency(task, &node, stamper);
    let (output, was_executed) = self.make_task_consistent(task, node);
    self.session.update_reserved_task_require_dependency(&node, output.clone());

    self.session.tracker.require_task_end(task, &output, was_executed);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> MakeConsistent<T> for TopDownContext<'p, 's, T, T::Output, A, H> {
  #[inline]
  fn make_task_consistent(&mut self, task: &T) -> T::Output {
    let node = self.session.store.get_or_create_task_node(task);
    let (output, _) = self.make_task_consistent(task, node);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  /// Make `task` (with corresponding `node`) consistent, returning its output and whether it was executed.
  #[inline]
  fn make_task_consistent(&mut self, task: &T, node: TaskNode) -> (T::Output, bool) {
    let already_consistent = self.session.consistent.contains(&node);
    let should_execute = !already_consistent && self.should_execute_task(task, &node);
    let output = if should_execute { // Execute the task, cache and return output.
      let previous_executing_task = self.session.pre_execute(task, node);
      let output = task.execute(self);
      self.session.post_execute(task, &node, previous_executing_task, output.clone());
      output
    } else { // Return consistent output.
      // If we should not execute the task, the store has an output for it. There are two possible cases:
      // - Not already consistent: `should_execute_task` returned `false` => store has an output for the task.
      // - Already consistent: previous `make_task_consistent` either executed the task and stored its output, or
      //   deemed it consistent => store has an output for the task.
      // In both cases the store has an output for the task, so `get_task_output` will not panic.
      self.session.store.get_task_output(&node).clone()
    };
    self.session.consistent.insert(node);
    (output, should_execute)
  }

  /// Checks whether `task` (with corresponding `node`) should be executed, returning `true` if it should be executed.
  #[inline]
  fn should_execute_task(&mut self, task: &T, node: &TaskNode) -> bool {
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

  /// Checks whether `dependency` is inconsistent, returning `true` if it is inconsistent.
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
