use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use crate::{Context, Session, Task};
use crate::context::ContextShared;
use crate::dependency::{Dependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies recursively in a top-down manner.
pub struct TopDownContext<'p, 's, T, O, A, H> {
  shared: ContextShared<'p, 's, T, O, A, H>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  /// Creates a new [`TopDownRunner`] with given [`Tracker`].
  #[inline]
  pub fn new(session: &'s mut Session<'p, T, T::Output, A, H>) -> Self {
    Self {
      shared: ContextShared::new(session),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    self.shared.reset();
    self.shared.session.tracker.require_top_down_initial_start(task);
    let output = self.require_task(task);
    self.shared.session.tracker.require_top_down_initial_end(task, &output);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for TopDownContext<'p, 's, T, T::Output, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.shared.session.tracker.require_task(task);
    let node = self.shared.session.store.get_or_create_task_node(task);

    self.shared.reserve_task_require_dependency(&node, task);

    let output = if !self.shared.session.visited.contains(&node) && self.should_execute_task(&node, task) { // Execute the task, cache and return up-to-date output.
      let previous_executing_task = self.shared.pre_execute(node, task);
      let output = task.execute(self);
      self.shared.post_execute(previous_executing_task, node, task, output.clone());
      output
    } else { // Return already up-to-date output.
      self.shared.session.tracker.up_to_date(task);
      // No panic: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.shared.session.store.get_task_output(&node).clone();
      output
    };

    let dependency = TaskDependency::new(task.clone(), stamper, output.clone());
    self.shared.update_reserved_task_require_dependency(&node, dependency);
    self.shared.session.visited.insert(node);

    output
  }

  #[inline]
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, std::io::Error> {
    self.shared.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.shared.provide_file_with_stamper(path, stamper)
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { self.shared.default_output_stamper() }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { self.shared.default_require_file_stamper() }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { self.shared.default_provide_file_stamper() }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  /// Checks whether the task should be executed, returning `true` if it should be executed.
  fn should_execute_task(&mut self, node: &TaskNode, task: &T) -> bool {
    self.shared.session.tracker.check_top_down_start(task);

    let mut has_dependencies = false;
    let mut is_dependency_inconsistent = false;
    // Borrow: because we pass `&mut self` to `is_dependency_inconsistent` for recursive consistency checking, we need
    //         to clone and collect dependencies into a `Vec` here.
    let dependencies: Vec<_> = self.shared.session.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      has_dependencies = true;
      is_dependency_inconsistent |= self.is_dependency_inconsistent(dependency);
    }

    let should_execute = match has_dependencies {
      true => is_dependency_inconsistent, // If the task has dependencies, execute if a dependency is inconsistent.
      false => !self.shared.session.store.task_has_output(node), // If task has no dependencies, execute if it has no output, meaning that it has never been executed.
    };

    self.shared.session.tracker.check_top_down_end(task);
    should_execute
  }

  #[allow(clippy::wrong_self_convention)]
  #[inline]
  fn is_dependency_inconsistent(&mut self, dependency: Option<Dependency<T, T::Output>>) -> bool {
    let Some(dependency) = dependency else {
      panic!("BUG: checking reserved dependency for inconsistency");
    };
    self.shared.session.tracker.check_dependency_start(&dependency);
    let inconsistent = dependency.is_inconsistent(self);
    self.shared.session.tracker.check_dependency_end(&dependency, inconsistent.as_ref().map(|o| o.as_ref()));
    match inconsistent {
      Ok(Some(_)) => true,
      Err(e) => { // Error while checking: store error and assume inconsistent
        self.shared.session.dependency_check_errors.push(e);
        true
      }
      _ => false,
    }
  }
}
