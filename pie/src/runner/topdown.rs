use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

use anymap::AnyMap;
use bimap::BiHashMap;
use incremental_topo::{IncrementalTopo, Node};

use crate::{Context, Dependency, DynTask, FileDependency, Task, TaskDependency};

/// Incremental runner that checks dependencies recursively in a top-down manner.
pub struct TopDownRunner {
  graph: IncrementalTopo,
  task_node: BiHashMap<Box<dyn DynTask>, Node>,
  file_to_node: HashMap<PathBuf, Node>,

  file_to_task_requires: HashMap<PathBuf, Vec<Node>>,
  file_to_task_provider: HashMap<PathBuf, Node>,

  task_outputs: AnyMap,
  task_dependencies: HashMap<Node, Vec<Box<dyn Dependency<Self>>>>,

  task_execution_stack: Vec<Node>,

  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl TopDownRunner {
  /// Creates a new `[TopDownRunner]`.
  pub fn new() -> Self {
    Self {
      graph: IncrementalTopo::new(),
      task_node: BiHashMap::new(),
      file_to_node: HashMap::new(),

      file_to_task_requires: HashMap::new(),
      file_to_task_provider: HashMap::new(),

      task_outputs: AnyMap::new(),
      task_dependencies: HashMap::new(),

      task_execution_stack: Vec::new(),

      dependency_check_errors: Vec::new(),
    }
  }

  /// Requires given `[task]`, returning its up-to-date output, or an error indicating failure to check consistency of 
  /// one or more dependencies.
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
    let task_node = self.get_task_node(Box::new(task.clone()) as Box<dyn DynTask>);
    if let Some(current_task_node) = self.task_execution_stack.last() {
      if let Err(incremental_topo::Error::CycleDetected) = self.graph.add_dependency(current_task_node, task_node) {
        let current_task = self.task_node.get_by_right(&current_task_node);
        panic!("Cyclic task dependency; task {:?} required task {:?} which was already required. Task stack: {:?}", current_task, task, self.task_execution_stack);
      }
    }
    if self.should_execute_task(task_node) {
      self.task_execution_stack.push(task_node);
      // TODO: remove task from file_to_task_requires and file_to_task_provider by going over dependencies
      let output = task.execute(self);
      self.task_execution_stack.pop();
      if let Some(current_task_node) = self.task_execution_stack.last() {
        self.add_to_task_dependencies(*current_task_node, Box::new(TaskDependency::new(task.clone(), output.clone())));
      }
      self.set_task_output(task.clone(), output.clone());
      output
    } else {
      // Assume: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.get_task_output::<T>(task).unwrap().clone();
      output
    }
  }

  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> {
    let file_node = self.get_file_node(path);
    // TODO: hidden dependency detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_task_node) = self.task_execution_stack.last() {
      self.graph.add_dependency(*current_task_node, file_node).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
      self.add_to_task_dependencies(*current_task_node, Box::new(dependency));
      // TODO: add to file_to_task_requires
    }
    opened
  }

  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error> {
    let file_node = self.get_file_node(path);
    // TODO: hidden dependency detection
    // TODO: overlapping provided file detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    if let Some(current_task_node) = self.task_execution_stack.last() {
      self.graph.add_dependency(*current_task_node, file_node).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
      self.add_to_task_dependencies(*current_task_node, Box::new(dependency));
      // TODO: set file_to_task_provider
    }
    Ok(())
  }
}

impl TopDownRunner {
  fn should_execute_task(&mut self, task_node: Node) -> bool {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.remove_task_dependencies(&task_node);
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
      self.set_task_dependencies(task_node, task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      false
    } else {
      // Task has not been executed before, therefore we need to execute it.
      true
    }
  }


  #[inline]
  fn get_task_node(&mut self, task: Box<dyn DynTask>) -> Node {
    if let Some(node) = self.task_node.get_by_left(&task) {
      *node
    } else {
      let node = self.graph.add_node();
      self.task_node.insert(task, node);
      node
    }
  }
  #[inline]
  fn get_file_node(&mut self, path: &PathBuf) -> Node {
    if let Some(file_node) = self.file_to_node.get(path) {
      *file_node
    } else {
      let node = self.graph.add_node();
      self.file_to_node.insert(path.clone(), node);
      node
    }
  }

  #[inline]
  fn remove_task_dependencies(&mut self, task_node: &Node) -> Option<Vec<Box<dyn Dependency<Self>>>> {
    self.task_dependencies.remove(task_node)
  }
  #[inline]
  fn set_task_dependencies(&mut self, task_node: Node, dependencies: Vec<Box<dyn Dependency<Self>>>) {
    self.task_dependencies.insert(task_node, dependencies);
  }
  #[inline]
  fn add_to_task_dependencies(&mut self, task_node: Node, dependency: Box<dyn Dependency<Self>>) {
    let dependencies = self.task_dependencies.entry(task_node).or_insert_with(|| Vec::new());
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
