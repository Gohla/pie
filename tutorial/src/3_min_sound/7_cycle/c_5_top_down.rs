use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, fs, Session, Task};
use crate::dependency::{FileDependency, MakeConsistent, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;
use crate::tracker::Tracker;

pub struct TopDownContext<'p, 's, T, O, A> {
  session: &'s mut Session<'p, T, O, A>,
}

impl<'p, 's, T: Task, A: Tracker<T>> TopDownContext<'p, 's, T, T::Output, A> {
  pub fn new(session: &'s mut Session<'p, T, T::Output, A>) -> Self { Self { session } }

  pub fn require_initial(&mut self, task: &T) -> T::Output {
    self.session.tracker.build_start();
    let output = self.require_task(task);
    self.session.tracker.build_end();
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>> Context<T> for TopDownContext<'p, 's, T, T::Output, A> {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let Some(current_executing_task_node) = &self.session.current_executing_task else {
      return fs::open_if_file(path); // No current executing task, so no dependency needs to be made.
    };
    let path = path.as_ref();
    let node = self.session.store.get_or_create_file_node(path);

    if let Some(providing_task_node) = self.session.store.get_task_providing_file(&node) {
      if !self.session.store.contains_transitive_task_dependency(current_executing_task_node, &providing_task_node) {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        let providing_task = self.session.store.get_task(&providing_task_node);
        panic!("Hidden dependency; file '{}' is required by the current executing task '{:?}' without a dependency to \
                providing task: {:?}", path.display(), current_executing_task, providing_task);
      }
    }

    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.session.tracker.require_file_end(&dependency);
    self.session.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }

  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), io::Error> {
    let Some(current_executing_task_node) = &self.session.current_executing_task else {
      return Ok(()); // No current executing task, so no dependency needs to be made.
    };
    let path = path.as_ref();
    let node = self.session.store.get_or_create_file_node(path);

    if let Some(previous_providing_task_node) = self.session.store.get_task_providing_file(&node) {
      let current_executing_task = self.session.store.get_task(current_executing_task_node);
      let previous_providing_task = self.session.store.get_task(&previous_providing_task_node);
      panic!("Overlapping provided file; file '{}' is provided by the current executing task '{:?}' that was \
              previously provided by task: {:?}", path.display(), current_executing_task, previous_providing_task);
    }

    for requiring_task_node in self.session.store.get_tasks_requiring_file(&node) {
      if !self.session.store.contains_transitive_task_dependency(&requiring_task_node, current_executing_task_node) {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        let requiring_task = self.session.store.get_task(&requiring_task_node);
        panic!("Hidden dependency; file '{}' is provided by the current executing task '{:?}' without a dependency \
                from requiring task '{:?}' to the current executing task", path.display(), current_executing_task, requiring_task);
      }
    }

    let dependency = FileDependency::new(path, stamper)?;
    self.session.tracker.provide_file_end(&dependency);
    self.session.store.add_file_provide_dependency(current_executing_task_node, &node, dependency);
    Ok(())
  }

  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.session.tracker.require_task_start(task, &stamper);

    let node = self.session.store.get_or_create_task_node(task);
    if let Some(current_executing_task_node) = &self.session.current_executing_task {
      // First reserve a task require dependency to catch cycles before (potentially) executing the task, and to have
      // the dependency edge in the graph for catching future cycles.
      if self.session.store.reserve_task_require_dependency(current_executing_task_node, &node).is_err() {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which was already required", current_executing_task, task);
      }
    }
    let (output, was_executed) = self.make_task_consistent(task, node);

    let dependency = TaskDependency::new(task.clone(), stamper, output.clone());
    self.session.tracker.require_task_end(&dependency, &output, was_executed);

    if let Some(current_executing_task_node) = &self.session.current_executing_task {
      // Update the reserved task require dependency to a real task require dependency.
      self.session.store.update_task_require_dependency(current_executing_task_node, &node, dependency)
    }

    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>> MakeConsistent<T> for TopDownContext<'p, 's, T, T::Output, A> {
  fn make_task_consistent(&mut self, task: &T) -> T::Output {
    let node = self.session.store.get_or_create_task_node(task);
    let (output, _) = self.make_task_consistent(task, node);
    output
  }
}

impl<'p, 's, T: Task, A: Tracker<T>> TopDownContext<'p, 's, T, T::Output, A> {
  // Get required task output by executing it if needed, or by getting the output from the store if not needed.
  fn make_task_consistent(&mut self, task: &T, node: TaskNode) -> (T::Output, bool) {
    let already_consistent = self.session.consistent.contains(&node);
    let should_execute = !already_consistent && self.should_execute_task(&node);
    let output = if should_execute {
      self.session.tracker.execute_start(task);
      self.session.store.reset_task(&node);
      let previous_executing_task = self.session.current_executing_task.replace(node);
      let output = task.execute(self);
      self.session.current_executing_task = previous_executing_task;
      self.session.store.set_task_output(&node, output.clone());
      self.session.tracker.execute_end(task, &output);
      output
    } else {
      // Correctness: when `should_execute_task` returns `true`, the above block is executed. Otherwise this block is
      // executed and `should_execute_task` ensures that the task has an output.
      self.session.store.get_task_output(&node).clone()
    };

    self.session.consistent.insert(node);
    (output, should_execute)
  }

  /// Checks whether given task should be executed, returning `true` if it should be executed. A task should be executed
  /// if any of its dependencies are inconsistent, or when it has no output.
  fn should_execute_task(&mut self, node: &TaskNode) -> bool {
    // Borrow: because we pass `self` (which is `&mut self`) to `is_inconsistent` for recursive consistency checking,
    //         we need to clone and collect dependencies into a `Vec`. Otherwise we have an immutable borrow of `self`
    //         through `self.store` while we create a mutable borrow of `self`, which is not allowed.
    let dependencies: Vec<_> = self.session.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      self.session.tracker.check_dependency_start(&dependency);
      let inconsistency = dependency.is_inconsistent(self);
      self.session.tracker.check_dependency_end(&dependency, inconsistency.as_ref().map(|o| o.as_ref()));
      match inconsistency {
        Ok(Some(_)) => return true,
        Err(e) => { // Error while checking: store error and assume inconsistent
          self.session.dependency_check_errors.push(e);
          return true;
        }
        _ => {} // Consistent: continue checking
      }
    }
    // Task has no dependencies or all dependencies are consistent. Should only execute if it has no output, meaning
    // that it has never been executed before.
    return !self.session.store.task_has_output(node);
  }
}
