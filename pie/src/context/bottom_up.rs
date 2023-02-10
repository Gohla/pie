#![allow(unused_variables, dead_code)]

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::hash::BuildHasher;
use std::path::PathBuf;

use crate::{Context, Session, Task, TaskNodeId};
use crate::context::ContextShared;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{FileNodeId, Store};
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
  pub(crate) fn update_affected_by<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I) {
    self.shared.session.tracker.update_affected_by_start(changed_files.clone());

    // Create a new queue of scheduled tasks.
    self.scheduled = Queue::new();
    // Schedule affected tasks that require or provide a changed file.
    for path in changed_files {
      let file_node_id = self.shared.session.store.get_or_create_file_node(&path);
      Self::schedule_affected_by_file(
        &file_node_id,
        path,
        true,
        &self.shared.session.store,
        &mut self.shared.session.tracker,
        &mut self.shared.session.dependency_check_errors,
        &mut self.scheduled,
      );
    }
    // Execute the top scheduled task in the queue until it is empty.
    while let Some(task_node) = self.scheduled.pop(&mut self.shared.session.store) {
      self.execute_and_schedule(task_node);
    }

    self.shared.session.tracker.update_affected_by_end();
  }

  fn schedule_affected_by_file(
    file_node_id: &FileNodeId,
    path: &PathBuf,
    providing: bool,
    store: &Store<T, H>, // Passing in borrows explicitly instead of mutibly borrowing `self` to make borrows work.
    tracker: &mut A,
    dependency_check_errors: &mut Vec<Box<dyn Error>>,
    scheduled: &mut Queue<H>,
  ) {
    tracker.schedule_affected_by_file_start(path);
    for (requiring_task_node, dependency) in store.get_tasks_requiring_or_providing_file(file_node_id, providing) {
      let inconsistent = dependency.is_inconsistent();
      let requiring_task = store.task_by_node(requiring_task_node);
      tracker.check_affected_by_file(requiring_task, dependency, inconsistent.as_ref().map_err(|e| e.as_ref()).map(|o| o.as_ref()));
      match inconsistent {
        Err(e) => {
          dependency_check_errors.push(e);
          scheduled.add(*requiring_task_node);
        }
        Ok(Some(_)) => { // Schedule task; can't extract method due to self borrow above.
          let task = store.task_by_node(requiring_task_node);
          tracker.schedule_task(task);
          scheduled.add(*requiring_task_node);
        }
        _ => {}
      }
    }
    tracker.schedule_affected_by_file_end(path);
  }

  fn execute_and_schedule(&mut self, task_node_id: TaskNodeId) -> T::Output {
    // Execute `task`
    let task = self.shared.session.store.task_by_node(&task_node_id).clone(); // TODO: get rid of clone?
    let output = self.execute(task_node_id, &task);

    // Schedule affected tasks that require files provided by `task`.
    for provided_file in self.shared.session.store.get_provided_files(&task_node_id).copied() {
      let path = self.shared.session.store.path_by_node(&provided_file);
      Self::schedule_affected_by_file(
        &provided_file,
        path,
        false,
        &self.shared.session.store,
        &mut self.shared.session.tracker,
        &mut self.shared.session.dependency_check_errors,
        &mut self.scheduled,
      );
    }

    // Schedule affected tasks that require `task`'s output.
    self.shared.session.tracker.schedule_affected_by_task_start(&task);
    for (requiring_task_node, dependency) in self.shared.session.store.get_tasks_requiring_task(&task_node_id) {
      let inconsistent = dependency.is_inconsistent_with(output.clone());
      let requiring_task = self.shared.session.store.task_by_node(requiring_task_node);
      self.shared.session.tracker.check_affected_by_require_task(requiring_task, dependency, inconsistent.as_ref());
      if let Some(_) = inconsistent { // TODO: get rid of clone
        // Schedule task; can't extract method due to self borrow above.
        let task = self.shared.session.store.task_by_node(requiring_task_node);
        self.shared.session.tracker.schedule_task(task);
        self.scheduled.add(*requiring_task_node);
      }
    }
    self.shared.session.tracker.schedule_affected_by_task_end(&task);

    output
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
    while self.scheduled.is_not_empty() {
      if let Some(min_task_node_id) = self.scheduled.pop_least_task_with_dependency_to(task_node_id, &self.shared.session.store) {
        let output = self.execute_and_schedule(min_task_node_id);
        if min_task_node_id == *task_node_id {
          return Some(output);
        }
      }
    }
    None
  }
}


impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for IncrementalBottomUpContext<'p, 's, T, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    let task_node_id = self.shared.session.store.get_or_create_node_by_task(task.clone());

    if self.shared.session.visited.contains(&task_node_id) {
      // Unwrap OK: if we have already visited the task this session, it must have an output.
      let output = self.shared.session.store.get_task_output(&task_node_id).unwrap().clone();
      return output;
    }

    if !self.shared.session.store.task_has_output(&task_node_id) {
      // Have not executed the task before, so we just execute it.
      return self.execute(task_node_id, task);
    }

    // Task has an output, thus it has been executed before, therefore it has been scheduled if affected, or not 
    // scheduled if not affected. 

    if let Some(output) = self.require_scheduled_now(&task_node_id) {
      // Task was scheduled. That is, it was either directly or indirectly affected. Therefore, it has been
      // executed, and we return the result of that execution.
      output
    } else {
      // Task was not scheduled. That is, it was not directly affected by resource changes, and not indirectly
      // affected by other tasks. Therefore, we did not execute it.
      // TODO: can it be scheduled later, invalidating this claim? If not, explain why.

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


// Dependency ordered priority queue implementation

#[derive(Default, Debug)]
struct Queue<H> {
  set: HashSet<TaskNodeId, H>,
  vec: Vec<TaskNodeId>,
}

impl<H: BuildHasher + Default> Queue<H> {
  #[inline]
  fn new() -> Self { Self::default() }

  /// Checks whether the queue is not empty.
  #[inline]
  fn is_not_empty(&self) -> bool { !self.vec.is_empty() }

  /// Add a task to the priority queue. Does nothing if the task is already in the queue.
  #[inline]
  fn add(&mut self, task_node_id: TaskNodeId) {
    if self.set.contains(&task_node_id) { return; }
    self.set.insert(task_node_id);
    self.vec.push(task_node_id);
  }

  /// Remove the last task (task with the least amount of dependencies to other tasks in the queue) from the queue and
  /// return it.
  #[inline]
  fn pop<T: Task>(&mut self, store: &Store<T, H>) -> Option<TaskNodeId> {
    self.sort_by_dependencies(store);
    self.vec.pop()
  }

  /// Remove the last task (task with the least amount of dependencies to other tasks in the queue) that has a
  /// (transitive) dependency to given task, and return it.
  #[inline]
  fn pop_least_task_with_dependency_to<T: Task>(&mut self, dependee: &TaskNodeId, store: &Store<T, H>) -> Option<TaskNodeId> {
    self.sort_by_dependencies(store);
    let tasks: Vec<(usize, &TaskNodeId)> = self.vec.iter().enumerate().rev().collect(); // TODO: remove copy?
    for (idx, depender) in tasks {
      if store.contains_transitive_task_dependency(depender, dependee) {
        return Some(self.vec.swap_remove(idx));
      }
    }
    None
  }

  #[inline]
  fn sort_by_dependencies<T: Task>(&mut self, store: &Store<T, H>) {
    self.vec.sort_unstable_by(|n1, n2| store.graph.topo_cmp(n1, n2));
  }
} 
