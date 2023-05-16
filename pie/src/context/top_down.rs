use std::cell::Cell;
use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use pie_graph::Node;

use crate::{Context, Session, Task};
use crate::context::ContextShared;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies recursively in a top-down manner.
pub struct TopDownContext<'p, 's, T, O, A, H> {
  shared: ContextShared<'p, 's, T, O, A, H>,
  task_dependees_cache: Cell<Vec<Node>>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> TopDownContext<'p, 's, T, T::Output, A, H> {
  /// Creates a new [`TopDownRunner`] with given [`Tracker`].
  #[inline]
  pub fn new(session: &'s mut Session<'p, T, T::Output, A, H>) -> Self {
    Self {
      shared: ContextShared::new(session),
      task_dependees_cache: Cell::default(),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    self.shared.task_execution_stack.clear();
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

    self.shared.reserve_task_require_dependency(task, &node);

    let output = if !self.shared.session.visited.contains(&node) && self.should_execute_task(&node, task) { // Execute the task, cache and return up-to-date output.
      self.shared.session.store.reset_task(&node);
      self.shared.pre_execute(task, node);
      let output = task.execute(self);
      self.shared.post_execute(task, node, &output);
      output
    } else { // Return already up-to-date output.
      self.shared.session.tracker.up_to_date(task);
      // No panic: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.shared.session.store.get_task_output(&node).clone();
      output
    };

    self.shared.update_reserved_task_require_dependency(task.clone(), &node, output.clone(), stamper);
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
  fn should_execute_task(&mut self, node: &TaskNode, task: &T) -> bool {
    self.shared.session.tracker.check_top_down_start(task);

    // PERF: because this function can be recursively called, this cache (allocation) is not always reused. However, it
    //       is reused enough that it improves performance.
    let mut task_dependees = self.task_dependees_cache.take();
    task_dependees.clear();
    task_dependees.extend(self.shared.session.store.get_dependencies_of_task(node));

    // Check whether the dependencies are still consistent. If one or more are not, we need to execute the task.
    let mut has_dependencies = false;
    let mut is_dependency_inconsistent = false;
    for dependee in &task_dependees {
      has_dependencies = true;
      is_dependency_inconsistent |= self.is_dependency_inconsistent(node, dependee);
    }

    let result = if has_dependencies {
      is_dependency_inconsistent
    } else {
      if self.shared.session.store.task_has_output(node) {
        // Task has no dependencies; but has been executed before, so it never has to be executed again.
        false
      } else {
        // Task has not been executed, so we need to execute it.
        true
      }
    };

    self.task_dependees_cache.set(task_dependees);
    self.shared.session.tracker.check_top_down_end(task);
    result
  }

  #[allow(clippy::wrong_self_convention)]
  #[inline]
  fn is_dependency_inconsistent(&mut self, src: &TaskNode, dst: &Node) -> bool {
    let dependency = self.shared.session.store.get_dependency(src, dst);
    if let Some(dependency) = dependency {
      // Borrow: `dependency` is borrowed from the store (in `self`)`. Thus, we have to clone it because we pass `&mut self` to `is_inconsistent` later.
      let dependency = dependency.clone();
      self.shared.session.tracker.check_dependency_start(&dependency);
      let inconsistent = dependency.is_inconsistent(self);
      self.shared.session.tracker.check_dependency_end(&dependency, inconsistent.as_ref().map(|o| o.as_ref()));
      match inconsistent {
        Ok(Some(_)) => {
          return true;
        }
        Err(e) => { // Error while checking -> store error and assume not consistent.
          self.shared.session.dependency_check_errors.push(e);
          return true;
        }
        _ => {} // Continue to check other dependencies.
      }
    }
    false
  }
}
