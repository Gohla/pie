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

/// Context that incrementally executes tasks and checks dependencies recursively in a bottom-up manner.
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
    self.scheduled = Queue::new();
    self.schedule_affected_by_files(changed_files);
    while let Some(task_node) = self.scheduled.pop(&mut self.shared.session.store) {
      self.execute_and_schedule(task_node);
    }
  }


  fn schedule_affected_by_files(&mut self, changed_files: &[PathBuf]) {
    todo!()
  }

  fn schedule_affected_by_required_files(&mut self, required_files: &[PathBuf]) {
    todo!()
  }

  fn schedule_affected_by_required_task(&mut self, task: &T) {
    todo!()
  }

  fn execute_and_schedule(&mut self, task_node: TaskNodeId) {
    let task = self.shared.session.store.task_by_node(&task_node).clone(); // TODO: get rid of clone?
    self.shared.pre_execute(&task, task_node);
    let output = task.execute(self);
    self.shared.post_execute(&task, task_node, &output);
    for requiring_task_node in self.shared.session.store.get_task_nodes_requiring_task(&task_node) {
      todo!()
    }
  }
}


impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for IncrementalBottomUpContext<'p, 's, T, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    todo!()
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
} 
