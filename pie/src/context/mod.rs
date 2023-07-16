use std::fs::File;
use std::hash::BuildHasher;
use std::io;
use std::path::Path;

use crate::{Session, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

pub mod non_incremental;
pub mod top_down;
pub mod bottom_up;

/// Extension trait on [`Session`] for usage in [`Context`] implementations.
pub trait SessionExt<T, O> {
  /// Create a require file dependency with `stamper`, depending from the current executing task to file `path`.
  fn require_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<Option<File>, io::Error>;
  /// Create a provide file dependency with `stamper`, depending from the current executing task to file `path`.
  fn provide_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<(), io::Error>;

  /// Reserve a task require dependency, depending from the current executing task to `task` with node `dst`, detecting 
  /// cycles before we execute, preventing infinite recursion/loops. Does nothing if there is no current executing task.
  fn reserve_task_require_dependency(&mut self, task: &T, dst: &TaskNode, stamper: OutputStamper);
  /// Update the reserved task dependency, depending from the current executing task to `dst`, with `output`. Does 
  /// nothing if there is no current executing task.
  fn update_reserved_task_require_dependency(&mut self, dst: &TaskNode, output: O);

  /// Perform common pre-execution operations for `task` with `node`, returning the currently executing task.
  fn pre_execute(&mut self, task: &T, node: TaskNode) -> Option<TaskNode>;
  /// Perform common post-execution operations for `task` with `node`, restoring the `previous_executing_task` and 
  /// setting the `output` of the task.
  fn post_execute(&mut self, task: &T, node: &TaskNode, previous_executing_task: Option<TaskNode>, output: O);
}

impl<'p, T: Task, A: Tracker<T>, H: BuildHasher + Default> SessionExt<T, T::Output> for Session<'p, T, T::Output, A, H> {
  fn require_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let path = path.as_ref();
    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.tracker.require_file(&dependency);
    let node = self.store.get_or_create_file_node(path);
    let Some(current_executing_task_node) = &self.current_executing_task else {
      panic!("BUG: requiring file without a current executing task");
    };

    if let Some(dst) = self.store.get_task_providing_file(&node) {
      if !self.store.contains_transitive_task_dependency(current_executing_task_node, &dst) {
        let current_executing_task = self.store.get_task(current_executing_task_node);
        let dst_task = self.store.get_task(&dst);
        panic!("Hidden dependency; file '{}' is required by the current executing task '{:?}' without a dependency to providing task: {:?}", path.display(), current_executing_task, dst_task);
      }
    }

    self.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }
  fn provide_file_with_stamper(&mut self, path: impl AsRef<Path>, stamper: FileStamper) -> Result<(), io::Error> {
    let path = path.as_ref();
    let dependency = FileDependency::new(path, stamper).map_err(|e| e.kind())?;
    self.tracker.provide_file(&dependency);
    let node = self.store.get_or_create_file_node(path);
    let Some(current_executing_task_node) = &self.current_executing_task else {
      panic!("BUG: providing file without a current executing task");
    };

    if let Some(previous_providing_task_node) = self.store.get_task_providing_file(&node) {
      let current_executing_task = self.store.get_task(current_executing_task_node);
      let previous_providing_task = self.store.get_task(&previous_providing_task_node);
      panic!("Overlapping provided file; file '{}' is provided by the current executing task '{:?}' that was previously provided by task: {:?}", path.display(), current_executing_task, previous_providing_task);
    }

    for (requiring_task_node, _) in self.store.get_tasks_requiring_file(&node) {
      if !self.store.contains_transitive_task_dependency(&requiring_task_node, current_executing_task_node) {
        let current_executing_task = self.store.get_task(current_executing_task_node);
        let requiring_task = self.store.get_task(&requiring_task_node);
        panic!("Hidden dependency; file '{}' is provided by the current executing task '{:?}' without a dependency from requiring task '{:?}' to the current executing task", path.display(), current_executing_task, requiring_task);
      }
    }

    self.store.add_file_provide_dependency(current_executing_task_node, &node, dependency);
    Ok(())
  }


  #[inline]
  fn reserve_task_require_dependency(&mut self, task: &T, dst: &TaskNode, stamper: OutputStamper) {
    let Some(current_executing_task_node) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to reserve.
    };
    if let Err(()) = self.store.add_task_require_dependency(current_executing_task_node, dst, TaskDependency::new_reserved(task.clone(), stamper)) {
      let current_executing_task = self.store.get_task(current_executing_task_node);
      panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which directly or indirectly requires the current executing task", current_executing_task, &task);
    }
  }
  #[inline]
  fn update_reserved_task_require_dependency(&mut self, dst: &TaskNode, output: T::Output) {
    let Some(current_executing_task_node) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to update.
    };
    self.store.get_task_require_dependency_mut(current_executing_task_node, dst).update_reserved(output);
  }


  #[inline]
  fn pre_execute(&mut self, task: &T, node: TaskNode) -> Option<TaskNode> {
    // Note: pre/post execute methods are needed instead of one execute method because `&mut Context` needs to be passed 
    //       to `task.execute`, complicating borrowing.
    self.store.reset_task(&node);
    self.tracker.execute_task_start(task);
    self.current_executing_task.replace(node)
  }

  #[inline]
  fn post_execute(&mut self, task: &T, node: &TaskNode, previous_executing_task: Option<TaskNode>, output: T::Output) {
    self.current_executing_task = previous_executing_task;
    self.tracker.execute_task_end(task, &output);
    self.store.set_task_output(node, output);
  }
}
