use std::fs::File;
use std::hash::BuildHasher;
use std::path::Path;

use crate::{Session, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

pub mod non_incremental;
pub mod bottom_up;
pub mod top_down;

struct ContextShared<'p, 's, T, O, A, H> {
  pub session: &'s mut Session<'p, T, O, A, H>,
  current_executing_task: Option<TaskNode>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> ContextShared<'p, 's, T, T::Output, A, H> {
  pub fn new(session: &'s mut Session<'p, T, T::Output, A, H>) -> Self {
    Self {
      session,
      current_executing_task: None,
    }
  }

  pub fn reset(&mut self) {
    self.current_executing_task = None;
  }

  pub fn require_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<Option<File>, std::io::Error> {
    let path = path.as_ref();
    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.session.tracker.require_file(&dependency);
    let node = self.session.store.get_or_create_file_node(path);
    let Some(current_executing_task_node) = &self.current_executing_task else {
      panic!("BUG: requiring file without a current executing task");
    };

    if let Some(dst) = self.session.store.get_task_providing_file(&node) {
      if !self.session.store.contains_transitive_task_dependency(current_executing_task_node, dst) {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        let dst_task = self.session.store.get_task(dst);
        panic!("Hidden dependency; file '{}' is required by the current executing task '{:?}' without a dependency to providing task: {:?}", path.display(), current_executing_task, dst_task);
      }
    }

    self.session.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }

  pub fn provide_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    let dependency = FileDependency::new(path, stamper).map_err(|e| e.kind())?;
    self.session.tracker.provide_file(&dependency);
    let node = self.session.store.get_or_create_file_node(path);
    let Some(current_executing_task_node) = &self.current_executing_task else {
      panic!("BUG: providing file without a current executing task");
    };

    if let Some(previous_providing_task_node) = self.session.store.get_task_providing_file(&node) {
      let current_executing_task = self.session.store.get_task(current_executing_task_node);
      let previous_providing_task = self.session.store.get_task(&previous_providing_task_node);
      panic!("Overlapping provided file; file '{}' is provided by the current executing task '{:?}' that was previously provided by task: {:?}", path.display(), current_executing_task, previous_providing_task);
    }

    for (requiring_task_node, _) in self.session.store.get_tasks_requiring_file(&node) {
      if !self.session.store.contains_transitive_task_dependency(&requiring_task_node, current_executing_task_node) {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        let requiring_task = self.session.store.get_task(&requiring_task_node);
        panic!("Hidden dependency; file '{}' is provided by the current executing task '{:?}' without a dependency from requiring task '{:?}' to the current executing task", path.display(), current_executing_task, requiring_task);
      }
    }

    self.session.store.add_file_provide_dependency(current_executing_task_node, &node, dependency);
    Ok(())
  }

  /// Reserve a task require dependency from the current executing task to `dst`, detecting cycles before we execute, 
  /// preventing infinite recursion/loops.
  #[inline]
  pub fn reserve_task_require_dependency(&mut self, dst: &TaskNode, dst_task: &T) {
    let Some(current_executing_task_node) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to reserve.
    };
    if let Err(pie_graph::Error::CycleDetected) = self.session.store.reserve_task_require_dependency(current_executing_task_node, dst) {
      let current_executing_task = self.session.store.get_task(current_executing_task_node);
      panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which directly or indirectly requires the current executing task", current_executing_task, dst_task);
    }
  }

  /// Update the reserved task dependency from the current executing task to `dst` with an actual task dependency.
  #[inline]
  pub fn update_reserved_task_require_dependency(&mut self, dst: &TaskNode, dependency: TaskDependency<T, T::Output>) {
    let Some(current_executing_task_node) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to update.
    };
    self.session.store.update_reserved_task_require_dependency(current_executing_task_node, dst, dependency);
  }

  /// Perform common pre-execution operations. Pre/post execute methods needed instead of one execute method because 
  /// `&mut Context` needs to be passed to `task.execute`, complicating borrowing.
  #[inline]
  pub fn pre_execute(&mut self, node: TaskNode, task: &T) -> Option<TaskNode> {
    self.session.store.reset_task(&node);
    self.session.tracker.execute_task_start(task);
    self.current_executing_task.replace(node)
  }

  /// Perform common post-execution operations. Pre/post execute methods needed instead of one execute method because 
  /// `&mut Context` needs to be passed to `task.execute`, complicating borrowing.
  #[inline]
  pub fn post_execute(&mut self, previous_executing_task: Option<TaskNode>, node: TaskNode, task: &T, output: T::Output) {
    self.current_executing_task = previous_executing_task;
    self.session.tracker.execute_task_end(task, &output);
    self.session.store.set_task_output(&node, output);
  }

  #[inline]
  pub fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
  #[inline]
  pub fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  pub fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
}
