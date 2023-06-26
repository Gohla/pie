#![allow(unused_variables, dead_code)]

use std::collections::HashSet;
use std::fs::File;
use std::hash::BuildHasher;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Context, Session, Task, TaskNode};
use crate::context::SessionExt;
use crate::dependency::TaskDependency;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{FileNode, Store};
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies in a bottom-up manner.
pub(crate) struct BottomUpContext<'p, 's, T, O, A, H> {
  session: &'s mut Session<'p, T, O, A, H>,
  scheduled: Queue<H>,
  executing: HashSet<TaskNode, H>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> BottomUpContext<'p, 's, T, T::Output, A, H> {
  #[inline]
  pub(crate) fn new(session: &'s mut Session<'p, T, T::Output, A, H>) -> Self {
    Self {
      session,
      scheduled: Queue::new(),
      executing: HashSet::default(),
    }
  }

  /// Execute (i.e., make up-to-date or make consistent) all tasks affected by `changed_files`.
  #[inline]
  pub(crate) fn update_affected_by<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I) {
    self.session.reset();
    self.session.tracker.update_affected_by_start(changed_files.clone());

    // Create a new queue of scheduled tasks.
    self.scheduled = Queue::new();
    // Schedule affected tasks that require or provide a changed file.
    for path in changed_files {
      let file_node_id = self.session.store.get_or_create_file_node(&path);
      Self::schedule_affected_by_file(
        &file_node_id,
        path,
        true,
        &self.session.store,
        &mut self.session.tracker,
        &mut self.session.dependency_check_errors,
        &mut self.scheduled,
        &self.executing,
      );
    }
    // Execute the top scheduled task in the queue until it is empty.
    while let Some(node) = self.scheduled.pop(&mut self.session.store) {
      self.execute_and_schedule(node);
    }

    self.session.tracker.update_affected_by_end();
  }

  /// Schedule tasks affected by a change in the file at given `path`.
  fn schedule_affected_by_file(
    node: &FileNode,
    path: &PathBuf,
    providing: bool,
    store: &Store<T, T::Output, H>, // Passing in borrows explicitly instead of mutibly borrowing `self` to make borrows work.
    tracker: &mut A,
    dependency_check_errors: &mut Vec<io::Error>,
    scheduled: &mut Queue<H>,
    executing: &HashSet<TaskNode, H>,
  ) {
    tracker.schedule_affected_by_file_start(path);
    for (requiring_task_node, dependency) in store.get_tasks_requiring_or_providing_file(node, providing) {
      if executing.contains(&requiring_task_node) {
        continue; // Don't schedule tasks that are already executing.
      }
      let requiring_task = store.get_task(&requiring_task_node);
      tracker.check_affected_by_file_start(requiring_task, dependency);
      let inconsistent = dependency.is_inconsistent();
      tracker.check_affected_by_file_end(requiring_task, dependency, inconsistent.as_ref().map(|o| o.as_ref()));
      match inconsistent {
        Err(e) => {
          dependency_check_errors.push(e);
          scheduled.add(requiring_task_node);
        }
        Ok(Some(_)) => { // Schedule task; can't extract method due to self borrow above.
          tracker.schedule_task(requiring_task);
          scheduled.add(requiring_task_node);
        }
        _ => {}
      }
    }
    tracker.schedule_affected_by_file_end(path);
  }

  /// Execute the task identified by `node`, and then schedule new tasks based on the dependencies of the task.
  fn execute_and_schedule(&mut self, node: TaskNode) -> T::Output {
    let task = self.session.store.get_task(&node).clone(); // TODO: get rid of clone?
    let output = self.execute(node, &task);

    // Schedule affected tasks that require files provided by `task`.
    for provided_file in self.session.store.get_provided_files(&node) {
      let path = self.session.store.get_file_path(&provided_file);
      Self::schedule_affected_by_file(
        &provided_file,
        path,
        false,
        &self.session.store,
        &mut self.session.tracker,
        &mut self.session.dependency_check_errors,
        &mut self.scheduled,
        &self.executing,
      );
    }

    // Schedule affected tasks that require `task`'s output.
    self.session.tracker.schedule_affected_by_task_start(&task);
    for (requiring_task_node, dependency) in self.session.store.get_tasks_requiring_task(&node) {
      if self.executing.contains(&requiring_task_node) {
        continue; // Don't schedule tasks that are already executing.
      }
      let requiring_task = self.session.store.get_task(&requiring_task_node);
      self.session.tracker.check_affected_by_required_task_start(requiring_task, dependency);
      let inconsistent = dependency.is_inconsistent_with(&output);
      self.session.tracker.check_affected_by_required_task_end(requiring_task, dependency, inconsistent.clone());
      if let Some(_) = inconsistent {
        // Schedule task; can't extract method due to self borrow above.
        self.session.tracker.schedule_task(requiring_task);
        self.scheduled.add(requiring_task_node);
      }
    }
    self.session.tracker.schedule_affected_by_task_end(&task);

    output
  }

  /// Execute given `task`.
  #[inline]
  fn execute(&mut self, node: TaskNode, task: &T) -> T::Output {
    let previous_executing_task = self.session.pre_execute(node, task);
    self.executing.insert(node);
    let output = task.execute(self);
    self.executing.remove(&node);
    self.session.post_execute(previous_executing_task, node, task, output.clone());
    self.session.visited.insert(node);
    output
  }

  /// Execute scheduled tasks (and schedule new tasks) that depend (indirectly) on the task identified by `node`, 
  /// and then execute that scheduled task. Returns `Some` output if the task was (eventually) scheduled and thus 
  /// executed, or `None` if it was not executed and thus not (eventually) scheduled.
  #[inline]
  fn require_scheduled_now(&mut self, node: &TaskNode) -> Option<T::Output> {
    while self.scheduled.is_not_empty() {
      if let Some(min_task_node) = self.scheduled.pop_least_task_with_dependency_from(node, &self.session.store) {
        let output = self.execute_and_schedule(min_task_node);
        if min_task_node == *node {
          return Some(output);
        }
      } else {
        break;
      }
    }
    None
  }

  /// Make given `task` consistent and return its output.
  #[inline]
  fn make_consistent(&mut self, task: &T, node: TaskNode) -> T::Output {
    if self.session.visited.contains(&node) {
      // No panic: if we have already visited the task this session, it must have an output.
      let output = self.session.store.get_task_output(&node).clone();
      return output;
    }

    if !self.session.store.task_has_output(&node) {
      // Have not executed the task before, so we just execute it.
      return self.execute(node, task);
    }

    // Task has an output, thus it has been executed before, therefore it has been scheduled if affected, or not 
    // scheduled if not affected.

    if let Some(output) = self.require_scheduled_now(&node) {
      // Task was scheduled. That is, it was either directly or indirectly affected. Therefore, it has been
      // executed, and we return the result of that execution.
      output
    } else {
      // Task was not scheduled. That is, it was not directly affected by resource changes, and not indirectly
      // affected by other tasks. 
      //
      // The task cannot be affected during this build. Consider if the task would be affected, this can only occur in 
      // 3 different ways:
      // 
      // 1. the task is affected by a change in one of its require file dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent due to the absence of hidden dependencies.
      // 2. the task is affected by a change in one of its provided file dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent due to the absence of hidden dependencies
      //    and overlapping provided files.
      // 3. the task is affected by a change in one of its require task dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent because `require_scheduled_now` has made
      //    the task and all its (indirect) dependencies consistent.
      // 
      // All case cannot occur, thus the task cannot be affected. Therefore, we don't have to execute the task.
      self.session.tracker.up_to_date(task);

      // No panic: we don't have to execute the task and an output exists.
      let output = self.session.store.get_task_output(&node).clone();
      output
    }
  }
}


impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for BottomUpContext<'p, 's, T, T::Output, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.session.tracker.require_task(task);
    let node = self.session.store.get_or_create_task_node(task);

    let dependency = TaskDependency::new_reserved(task.clone(), stamper);
    self.session.reserve_task_require_dependency(&node, task, dependency);

    let output = self.make_consistent(task, node);

    self.session.update_reserved_task_require_dependency(&node, output.clone());
    self.session.visited.insert(node);

    output
  }

  #[inline]
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    self.session.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), io::Error> {
    self.session.provide_file_with_stamper(path, stamper)
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
  set: HashSet<TaskNode, H>,
  vec: Vec<TaskNode>,
}

impl<H: BuildHasher + Default> Queue<H> {
  #[inline]
  fn new() -> Self { Self::default() }

  /// Checks whether the queue is not empty.
  #[inline]
  fn is_not_empty(&self) -> bool { !self.vec.is_empty() }

  /// Add a task to the priority queue. Does nothing if the task is already in the queue.
  #[inline]
  fn add(&mut self, node: TaskNode) {
    if self.set.contains(&node) { return; }
    self.set.insert(node);
    self.vec.push(node);
  }

  /// Remove the last task (task with the least amount of dependencies to other tasks in the queue) from the queue and
  /// return it.
  #[inline]
  fn pop<T: Task>(&mut self, store: &Store<T, T::Output, H>) -> Option<TaskNode> {
    self.sort_by_dependencies(store);
    if let r @ Some(node) = self.vec.pop() {
      self.set.remove(&node);
      r
    } else {
      None
    }
  }

  /// Return the least task (task with the least amount of dependencies to other tasks in the queue) that has a 
  /// (transitive) dependency from task `depender`.
  #[inline]
  fn pop_least_task_with_dependency_from<T: Task>(&mut self, depender: &TaskNode, store: &Store<T, T::Output, H>) -> Option<TaskNode> {
    self.sort_by_dependencies(store);
    let mut found = None;
    for (idx, dependee) in self.vec.iter().enumerate().rev() {
      if depender == dependee || store.contains_transitive_task_dependency(depender, dependee) {
        found = Some((idx, *dependee));
        break;
      }
    }
    if let Some((index, task_node_id)) = found {
      self.vec.swap_remove(index); // Note: this prevents allocation but would require resorting as it changes ordering.
      self.set.remove(&task_node_id);
      return Some(task_node_id);
    }
    None
  }

  #[inline]
  fn sort_by_dependencies<T: Task>(&mut self, store: &Store<T, T::Output, H>) {
    // TODO: only sort if needed? Removing elements should not require a resort?
    // TODO: use select_nth_unstable_by(0) to get the sorted top element for pop?
    self.vec.sort_unstable_by(|node_a, node_b| store.topologically_compare(node_a, node_b));
  }
} 
