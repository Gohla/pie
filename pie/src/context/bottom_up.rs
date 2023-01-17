#![allow(unused_variables, dead_code)]

use std::collections::HashSet;
use std::fs::File;
use std::hash::BuildHasher;
use std::path::PathBuf;

use crate::{Context, Session, Task, TaskNodeId};
use crate::context::ContextShared;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::Store;
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies in a bottom-up manner.
#[derive(Debug)]
pub(crate) struct IncrementalBottomUpContext<'p, 's, T: Task, A, H> {
  shared: ContextShared<'p, 's, T, A, H>,
  scheduled: Queue<H>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> IncrementalBottomUpContext<'p, 's, T, A, H> {
  #[inline]
  pub(crate) fn new(session: &'s mut Session<'p, T, A, H>) -> Self {
    Self {
      shared: ContextShared::new(session),
      scheduled: Queue::new(),
    }
  }

  #[inline]
  pub(crate) fn update_affected_by(&mut self, changed_files: &[PathBuf]) {
    // Create a new queue of scheduled tasks.
    self.scheduled = Queue::new();
    // Schedule affected tasks that require or provide a changed file.
    for changed_file in changed_files {
      let file_node = self.shared.session.store.get_or_create_file_node(changed_file);
      for (requiring_task_node, dependency) in self.shared.session.store.get_tasks_requiring_or_providing_file(&file_node) {
        match dependency.require_or_provide_file_is_consistent() {
          Err(e) => {
            self.shared.session.dependency_check_errors.push(e);
            self.scheduled.add(*requiring_task_node);
          }
          Ok(false) => self.scheduled.add(*requiring_task_node),
          _ => {}
        }
      }
    }
    // Execute the top scheduled task in the queue until it is empty.
    while let Some(task_node) = self.scheduled.pop(&mut self.shared.session.store) {
      self.execute_and_schedule(task_node);
    }
  }


  fn execute_and_schedule(&mut self, task_node_id: TaskNodeId) {
    let task = self.shared.session.store.task_by_node(&task_node_id).clone(); // TODO: get rid of clone?
    let output = self.execute(task_node_id, &task);
    // Schedule affected tasks that require `task`.
    for (requiring_task_node, dependency) in self.shared.session.store.get_tasks_requiring_task(&task_node_id) {
      if !dependency.require_task_is_consistent_with(output.clone()) { // TODO: get rid of clone
        self.scheduled.add(*requiring_task_node);
      }
    }
    // Schedule affected tasks that require files provided by `task`.
    for provided_file in self.shared.session.store.get_provided_files(&task_node_id) {
      for (requiring_task_node, dependency) in self.shared.session.store.get_tasks_requiring_file(provided_file) {
        match dependency.require_or_provide_file_is_consistent() {
          Err(e) => {
            self.shared.session.dependency_check_errors.push(e);
            self.scheduled.add(*requiring_task_node);
          }
          Ok(false) => self.scheduled.add(*requiring_task_node),
          _ => {}
        }
      }
    }
  }

  fn execute(&mut self, task_node_id: TaskNodeId, task: &T) -> T::Output {
    self.shared.session.store.reset_task(&task_node_id);
    self.shared.pre_execute(&task, task_node_id);
    let output = task.execute(self);
    self.shared.post_execute(&task, task_node_id, &output);
    self.shared.session.visited.insert(task_node_id);
    output
  }

  fn require_scheduled_now(&mut self, task_node_id: &TaskNodeId) -> Option<T::Output> {
    todo!()
  }
}


impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for IncrementalBottomUpContext<'p, 's, T, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    let task_node_id = self.shared.session.store.get_or_create_node_by_task(task.clone());

    if self.shared.session.visited.contains(&task_node_id) {
      // Unwrap OK: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.shared.session.store.get_task_output(&task_node_id).unwrap().clone();
      return output;
    }

    if !self.shared.session.store.task_has_output(&task_node_id) {
      return self.execute(task_node_id, task);
    }

    // Task is in dependency graph, because we have stored data for it.

    if let Some(output) = self.require_scheduled_now(&task_node_id) {
      // Task was scheduled. That is, it was either directly or indirectly affected. Therefore, it has been
      // executed, and we return the result of that execution.
      output
    } else {
      // Task was not scheduled. That is, it was not directly affected by resource changes, and not indirectly
      // affected by other tasks. Therefore, we did not execute it.

      // Mark as visited
      self.shared.session.visited.insert(task_node_id);

      // Unwrap OK: we don't have to execute the task and an output exists.
      let output = self.shared.session.store.get_task_output(&task_node_id).unwrap().clone();
      return output;
    }
  }

  #[inline]
  fn require_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<File, std::io::Error> {
    self.shared.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.shared.provide_file_with_stamper(path, stamper)
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
}


// Queue implementation

#[derive(Default, Debug)]
struct Queue<H> {
  set: HashSet<TaskNodeId, H>,
  vec: Vec<TaskNodeId>,
}

impl<H: BuildHasher + Default> Queue<H> {
  #[inline]
  fn new() -> Self { Self::default() }

  #[inline]
  fn is_not_empty(&self) -> bool { !self.vec.is_empty() }

  #[inline]
  fn add(&mut self, task_node: TaskNodeId) {
    if self.set.contains(&task_node) { return; }
    self.set.insert(task_node);
    self.vec.push(task_node);
  }

  #[inline]
  fn pop<T: Task>(&mut self, store: &Store<T, H>) -> Option<TaskNodeId> {
    self.vec.sort_unstable_by(|n1, n2| store.graph.topo_cmp(n1, n2));
    self.vec.pop()
  }

  #[inline]
  fn poll_least_task_with_dependency_to(&self) {
    todo!()
  }
} 
