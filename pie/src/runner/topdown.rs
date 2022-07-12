use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

use anymap::AnyMap;
use dyn_clone::clone_box;
use hashlink::LinkedHashSet;

use crate::{Context, Dependency, DynTask, FileDependency, Task, TaskDependency};

/// Incremental runner that checks dependencies recursively in a top-down manner.
pub struct TopDownRunner {
  task_outputs: AnyMap,
  task_dependencies: HashMap<Box<dyn DynTask>, Vec<Box<dyn Dependency<Self>>>>,
  task_execution_stack: LinkedHashSet<Box<dyn DynTask>>,
  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl TopDownRunner {
  pub fn new() -> Self {
    Self {
      task_outputs: AnyMap::new(),
      task_dependencies: HashMap::new(),
      task_execution_stack: LinkedHashSet::new(),
      dependency_check_errors: Vec::new(),
    }
  }

  pub fn require_initial<T: Task>(&mut self, task: &T) -> Result<T::Output, (T::Output, &[Box<dyn Error>])> {
    self.task_execution_stack.clear();
    self.dependency_check_errors.clear();
    let output = self.require_task::<T>(task);
    if self.dependency_check_errors.is_empty() {
      Ok(output)
    } else {
      Err((output, &self.dependency_check_errors))
    }
  }
}

impl Context for TopDownRunner {
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output {
    let boxed_task = Box::new(task.clone()) as Box<dyn DynTask>;
    if self.task_execution_stack.contains(&boxed_task) {
      let current_task = self.task_execution_stack.back().unwrap(); // Unwrap OK: stack is not empty because it contains `boxed_task`.
      panic!("Cyclic task dependency; task {:?} required task {:?} which was already required. Task stack: {:?}", current_task, task, self.task_execution_stack);
    }
    if self.should_execute_task(task) {
      self.task_execution_stack.insert(boxed_task);
      let output = task.execute(self);
      self.task_execution_stack.pop_back();
      if let Some(current_task) = self.task_execution_stack.back() {
        self.add_to_task_dependencies(current_task.clone(), Box::new(TaskDependency::new(task.clone(), output.clone())));
      }
      self.set_task_output(task.clone(), output.clone());
      output
    } else {
      // Assume: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.get_task_output::<T>(task).unwrap().clone();
      output
    }
  }

  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> { // TODO: hidden dependency detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_task) = self.task_execution_stack.back() {
      self.add_to_task_dependencies(current_task.clone(), Box::new(dependency));
    }
    opened
  }

  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> { // TODO: hidden dependency detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_task) = self.task_execution_stack.back() {
      self.add_to_task_dependencies(current_task.clone(), Box::new(dependency));
    }
    opened
  }
}

impl TopDownRunner {
  fn should_execute_task(&mut self, task: &dyn DynTask) -> bool {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.remove_task_dependencies(task);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        match task_dependency.is_consistent(self) {
          Ok(false) => return true, // Not consistent -> should execute task.
          Err(e) => { // Error -> store error and assume not consistent -> should execute task.
            self.dependency_check_errors.push(e);
            return true;
          }
          _ => {}, // Continue to check other dependencies.
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.set_task_dependencies(clone_box(task), task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      false
    } else {
      // Task has not been executed before, therefore we need to execute it.
      true
    }
  }

  #[inline]
  fn remove_task_dependencies(&mut self, task: &dyn DynTask) -> Option<Vec<Box<dyn Dependency<Self>>>> {
    self.task_dependencies.remove(task)
  }
  #[inline]
  fn set_task_dependencies(&mut self, task: Box<dyn DynTask>, dependencies: Vec<Box<dyn Dependency<Self>>>) {
    self.task_dependencies.insert(task, dependencies);
  }
  #[inline]
  fn add_to_task_dependencies(&mut self, task: Box<dyn DynTask>, dependency: Box<dyn Dependency<Self>>) {
    let dependencies = self.task_dependencies.entry(task).or_insert_with(|| Vec::new());
    dependencies.push(dependency);
  }

  #[inline]
  fn get_task_output_map<T: Task>(&self) -> Option<&HashMap<T, T::Output>> {
    self.task_outputs.get::<HashMap<T, T::Output>>()
  }
  #[inline]
  fn get_task_output_map_mut<T: Task>(&mut self) -> &mut HashMap<T, T::Output> {
    self.task_outputs.entry::<HashMap<T, T::Output>>().or_insert_with(|| HashMap::default())
  }
  #[inline]
  fn get_task_output<T: Task>(&self, task: &T) -> Option<&T::Output> {
    self.get_task_output_map::<T>().map_or(None, |map| map.get(task))
  }
  #[inline]
  fn set_task_output<T: Task>(&mut self, task: T, output: T::Output) {
    self.get_task_output_map_mut::<T>().insert(task, output);
  }
}
