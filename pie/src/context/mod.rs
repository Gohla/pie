use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use crate::{Session, Task};
use crate::dependency::{Dependency, FileDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

pub(crate) mod non_incremental;
pub(crate) mod bottom_up;
pub(crate) mod top_down;

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

  fn require_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<Option<File>, std::io::Error> {
    let path = path.as_ref();
    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.session.tracker.require_file(&dependency);
    let file_node = self.session.store.get_or_create_file_node(path);
    if let Some(current_requiring_task_node) = self.task_execution_stack.last() {
      if let Some(providing_task_node) = self.session.store.get_task_providing_file(&file_node) {
        if !self.session.store.contains_transitive_task_dependency(current_requiring_task_node, &providing_task_node) {
          let current_requiring_task = self.session.store.task_by_node(current_requiring_task_node);
          let providing_task = self.session.store.task_by_node(&providing_task_node);
          panic!("Hidden dependency; file '{}' is required by the current task '{:?}' without a dependency to providing task: {:?}", path.display(), current_requiring_task, providing_task);
        }
      }
      self.session.store.add_file_require_dependency(current_requiring_task_node, &file_node, Dependency::require_file(dependency));
    }
    Ok(file)
  }

  fn provide_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    let dependency = FileDependency::new(path, stamper).map_err(|e| e.kind())?;
    self.session.tracker.provide_file(&dependency);
    let file_node = self.session.store.get_or_create_file_node(path);
    if let Some(current_providing_task_node) = self.task_execution_stack.last() {
      if let Some(previous_providing_task_node) = self.session.store.get_task_providing_file(&file_node) {
        let current_providing_task = self.session.store.task_by_node(current_providing_task_node);
        let previous_providing_task = self.session.store.task_by_node(&previous_providing_task_node);
        panic!("Overlapping provided file; file '{}' is provided by the current task '{:?}' that was previously provided by task: {:?}", path.display(), current_providing_task, previous_providing_task);
      }
      for (requiring_task_node, _) in self.session.store.get_tasks_requiring_file(&file_node) {
        if !self.session.store.contains_transitive_task_dependency(&requiring_task_node, current_providing_task_node) {
          let current_providing_task = self.session.store.task_by_node(current_providing_task_node);
          let requiring_task = self.session.store.task_by_node(&requiring_task_node);
          panic!("Hidden dependency; file '{}' is provided by the current task '{:?}' without a dependency from requiring task '{:?}' to the current providing task", path.display(), current_providing_task, requiring_task);
        }
      }
      self.session.store.add_file_provide_dependency(current_providing_task_node, &file_node, Dependency::provide_file(dependency));
    }
    Ok(())
  }

  /// Add dependency edge to graph, but without dependency data, as we first need to execute the task to get an output 
  /// to use as dependency data. This also detects cycles before we execute, preventing infinite recursion/loops.
  fn add_task_require_dependency(&mut self, task: &T, task_node_id: &TaskNode) {
    if let Some(current_task_node) = self.task_execution_stack.last() {
      if let Err(pie_graph::Error::CycleDetected) = self.session.store.add_to_dependencies_of_task(current_task_node, task_node_id, None) {
        let current_task = self.session.store.task_by_node(current_task_node);
        let task_stack: Vec<_> = self.task_execution_stack.iter().map(|task_node| self.session.store.task_by_node(task_node)).collect();
        panic!("Cyclic task dependency; current task '{:?}' is requiring task '{:?}' which was already required. Task stack: {:?}", current_task, task, task_stack);
      }
    }
  }

  fn update_task_require_dependency(&mut self, task: T, task_node_id: &TaskNode, output: T::Output, stamper: OutputStamper) {
    if let Some(current_task_node) = self.task_execution_stack.last() {
      let dependency = Dependency::require_task(task, output, stamper);
      self.session.store.update_dependency_of_task(current_task_node, task_node_id, Some(dependency));
    }
  }

  fn pre_execute(&mut self, task: &T, task_node_id: TaskNode) {
    self.task_execution_stack.push(task_node_id);
    self.session.tracker.execute_task_start(task);
  }

  fn post_execute(&mut self, task: &T, task_node_id: TaskNode, output: &T::Output) {
    self.session.tracker.execute_task_end(task, output);
    self.task_execution_stack.pop();
    self.session.store.set_task_output(&task_node_id, output.clone());
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
}
