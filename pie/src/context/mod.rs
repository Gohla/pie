use std::fs::File;
use std::hash::BuildHasher;
use std::path::PathBuf;

use crate::{Session, Task};
use crate::dependency::Dependency;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

pub(crate) mod bottom_up;
pub(crate) mod top_down;

#[derive(Debug)]
struct ContextShared<'p, 's, T: Task, A, H> {
  pub(crate) session: &'s mut Session<'p, T, A, H>,
  pub(crate) task_execution_stack: Vec<TaskNode>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> ContextShared<'p, 's, T, A, H> {
  fn new(session: &'s mut Session<'p, T, A, H>) -> Self {
    Self {
      session,
      task_execution_stack: Default::default(),
    }
  }

  fn require_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<File, std::io::Error> {
    self.session.tracker.require_file(path);
    let file_node = self.session.store.get_or_create_file_node(path);
    let (dependency, file) = Dependency::require_file(path, stamper)?;
    if let Some(current_requiring_task_node) = self.task_execution_stack.last() {
      if let Some(providing_task_node) = self.session.store.get_task_node_providing_file(&file_node) {
        if !self.session.store.contains_transitive_task_dependency(current_requiring_task_node, &providing_task_node) {
          let current_requiring_task = self.session.store.task_by_node(current_requiring_task_node);
          let providing_task = self.session.store.task_by_node(&providing_task_node);
          panic!("Hidden dependency; file '{}' is required by the current task '{:?}' without a dependency to providing task: {:?}", path.display(), current_requiring_task, providing_task);
        }
      }
      self.session.store.add_file_require_dependency(*current_requiring_task_node, file_node, dependency);
    }
    Ok(file)
  }

  fn provide_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.session.tracker.provide_file(path);
    let file_node = self.session.store.get_or_create_file_node(path);
    let dependency = Dependency::provide_file(path, stamper).map_err(|e| e.kind())?;
    if let Some(current_providing_task_node) = self.task_execution_stack.last() {
      if let Some(previous_providing_task_node) = self.session.store.get_task_node_providing_file(&file_node) {
        let current_providing_task = self.session.store.task_by_node(current_providing_task_node);
        let previous_providing_task = self.session.store.task_by_node(&previous_providing_task_node);
        panic!("Overlapping provided file; file '{}' is provided by the current task '{:?}' that was previously provided by task: {:?}", path.display(), current_providing_task, previous_providing_task);
      }
      for requiring_task_node in self.session.store.get_task_nodes_requiring_file(&file_node) {
        if !self.session.store.contains_transitive_task_dependency(&requiring_task_node, current_providing_task_node) {
          let current_providing_task = self.session.store.task_by_node(current_providing_task_node);
          let requiring_task = self.session.store.task_by_node(&requiring_task_node);
          panic!("Hidden dependency; file '{}' is provided by the current task '{:?}' without a dependency from requiring task '{:?}' to the current providing task", path.display(), current_providing_task, requiring_task);
        }
      }
      self.session.store.add_file_provide_dependency(*current_providing_task_node, file_node, dependency);
    }
    Ok(())
  }

  fn pre_execute(&mut self, task: &T, task_node: TaskNode) {
    self.task_execution_stack.push(task_node);
    self.session.tracker.execute_task_start(task);
  }

  fn post_execute(&mut self, task: &T, task_node: TaskNode, output: &T::Output) {
    self.session.tracker.execute_task_end(task, output);
    self.task_execution_stack.pop();
    self.session.store.set_task_output(task_node, output.clone());
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
}
