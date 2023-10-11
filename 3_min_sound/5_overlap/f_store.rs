use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

/// Stores files and tasks, and their dependencies, in a DAG (directed acyclic graph). Provides operations to mutate
/// and query this graph.
pub struct Store<T, O> {
  graph: DAG<NodeData<T, O>, Dependency<T, O>>,
  file_to_node: HashMap<PathBuf, FileNode>,
  task_to_node: HashMap<T, TaskNode>,
}

#[derive(Debug)]
enum NodeData<T, O> {
  File(PathBuf),
  Task {
    task: T,
    output: Option<O>,
  },
}

/// Newtype for file `Node`s.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileNode(Node);

impl Borrow<Node> for &FileNode {
  fn borrow(&self) -> &Node { &self.0 }
}

/// Newtype for task `Node`s.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskNode(Node);

impl Borrow<Node> for &TaskNode {
  fn borrow(&self) -> &Node { &self.0 }
}

impl<T: Task> Default for Store<T, T::Output> {
  fn default() -> Self {
    Self {
      graph: DAG::default(),
      file_to_node: HashMap::default(),
      task_to_node: HashMap::default(),
    }
  }
}

impl<T: Task> Store<T, T::Output> {
  /// Gets the file node for `path`, or creates a file node by adding it to the dependency graph.
  pub fn get_or_create_file_node(&mut self, path: impl AsRef<Path>) -> FileNode {
    let path = path.as_ref();
    if let Some(file_node) = self.file_to_node.get(path) {
      *file_node
    } else {
      let node = self.graph.add_node(NodeData::File(path.to_path_buf()));
      let node = FileNode(node);
      self.file_to_node.insert(path.to_path_buf(), node);
      node
    }
  }
  /// Gets the path for `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  #[allow(dead_code)]
  pub fn get_file_path(&self, node: &FileNode) -> &PathBuf {
    let Some(NodeData::File(path)) = self.graph.get_node_data(node) else {
      panic!("BUG: node {:?} was not found in the dependency graph", node);
    };
    path
  }

  /// Gets the task node for `task`, or creates a task node by adding it to the dependency graph.
  pub fn get_or_create_task_node(&mut self, task: &T) -> TaskNode {
    if let Some(node) = self.task_to_node.get(task) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Task {
        task: task.clone(),
        output: None,
      });
      let node = TaskNode(node);
      self.task_to_node.insert(task.clone(), node);
      node
    }
  }
  /// Gets the task for `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  pub fn get_task(&self, node: &TaskNode) -> &T {
    let Some(NodeData::Task { task, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node {:?} was not found in the dependency graph", node);
    };
    task
  }
}

impl<T: Task> Store<T, T::Output> {
  /// Checks whether task `node` has an output. Returns `false` if `node` does not have an output.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  pub fn task_has_output(&self, node: &TaskNode) -> bool {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node {:?} was not found in the dependency graph", node);
    };
    output.is_some()
  }
  /// Gets the output for task `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph, or if the task has no output.
  pub fn get_task_output(&self, node: &TaskNode) -> &T::Output {
    let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node {:?} was not found in the dependency graph, or does not have an output", node);
    };
    output
  }
  /// Sets the output for task `node` to `new_output`.
  ///
  /// # Panics
  ///
  /// Panics if task `node` was not found in the dependency graph.
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: T::Output) {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) else {
      panic!("BUG: node {:?} was not found in the dependency graph", node);
    };
    output.replace(new_output);
  }
}

impl<T: Task> Store<T, T::Output> {
  /// Get all dependencies of task `src`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` was not found in the dependency graph.
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Dependency<T, T::Output>> + 'a {
    debug_assert!(self.graph.contains_node(src), "BUG: node {:?} was not found in the dependency graph", src);
    self.graph.get_outgoing_edge_data(src)
  }
  /// Add a file require `dependency` from task `src` to file `dst`.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph, or if a cycle is created by adding this dependency.
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    match self.graph.add_edge(src, dst, Dependency::RequireFile(dependency)) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => panic!("BUG: cycle detected when adding file dependency from {:?} to {:?}", src, dst),
      _ => {},
    }
  }
  /// Add a file provide `dependency` from task `src` to file `dst`.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph, or if a cycle is created by adding this dependency.
  pub fn add_file_provide_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    match self.graph.add_edge(src, dst, Dependency::ProvideFile(dependency)) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => panic!("BUG: cycle detected when adding file dependency from {:?} to {:?}", src, dst),
      _ => {},
    }
  }
  /// Adds a task require `dependency` from task `src` to task `dst`.
  ///
  /// # Errors
  ///
  /// Returns `Err(())` if adding this dependency to the graph creates a cycle.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph.
  pub fn add_task_require_dependency(&mut self, src: &TaskNode, dst: &TaskNode, dependency: TaskDependency<T, T::Output>) -> Result<(), ()> {
    match self.graph.add_edge(src, dst, Dependency::RequireTask(dependency)) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => Err(()),
      _ => Ok(()),
    }
  }
}

impl<T: Task> Store<T, T::Output> {
  /// Reset task `src`, removing its output and removing all its outgoing dependencies.
  ///
  /// # Panics
  ///
  /// Panics if task `src` was not found in the dependency graph.
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(src) {
      *output = None;
    } else {
      panic!("BUG: node {:?} was not found in the dependency graph", src);
    }
    self.graph.remove_outgoing_edges_of_node(src);
  }
}


#[cfg(test)]
mod test {
  use crate::Context;
  use crate::stamp::{FileStamper, OutputStamper};

  use super::*;

  /// Task that returns its owned string. Never executed, just used for testing the store.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct StringConstant(String);

  impl StringConstant {
    pub fn new(string: impl Into<String>) -> Self { Self(string.into()) }
  }

  impl Task for StringConstant {
    type Output = String;
    fn execute<C: Context<Self>>(&self, _context: &mut C) -> Self::Output {
      self.0.clone()
    }
  }

  #[test]
  fn test_file_mapping() {
    let mut store: Store<StringConstant, String> = Store::default();

    let path_a = PathBuf::from("hello.txt");
    let node_a = store.get_or_create_file_node(&path_a);
    assert_eq!(node_a, store.get_or_create_file_node(&path_a)); // Same node
    assert_eq!(&path_a, store.get_file_path(&node_a)); // Same file path

    let path_b = PathBuf::from("world.txt");
    let node_b = store.get_or_create_file_node(&path_b);
    assert_eq!(node_b, store.get_or_create_file_node(&path_b));
    assert_eq!(&path_b, store.get_file_path(&node_b));

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  #[should_panic]
  fn test_file_mapping_panics() {
    let mut fake_store: Store<StringConstant, String> = Store::default();
    let fake_node = fake_store.get_or_create_file_node("hello.txt");
    let store: Store<StringConstant, String> = Store::default();
    store.get_file_path(&fake_node);
  }


  #[test]
  fn test_task_mapping() {
    let mut store = Store::default();

    let task_a = StringConstant::new("Hello");
    let node_a = store.get_or_create_task_node(&task_a);
    assert_eq!(node_a, store.get_or_create_task_node(&task_a)); // Same node
    assert_eq!(&task_a, store.get_task(&node_a)); // Same task

    let task_b = StringConstant::new("World");
    let node_b = store.get_or_create_task_node(&task_b);
    assert_eq!(node_b, store.get_or_create_task_node(&task_b));
    assert_eq!(&task_b, store.get_task(&node_b));

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  #[should_panic]
  fn test_task_mapping_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    store.get_task(&fake_node);
  }


  #[test]
  fn test_task_outputs() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(&output_a);
    let node_a = store.get_or_create_task_node(&task_a);

    let output_b = "World".to_string();
    let task_b = StringConstant::new(&output_b);
    let node_b = store.get_or_create_task_node(&task_b);

    // Assert that tasks have no output by default.
    assert!(!store.task_has_output(&node_a));
    assert!(!store.task_has_output(&node_b));

    // Set output for task A, assert that A has that output but B is unchanged.
    store.set_task_output(&node_a, output_a.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(!store.task_has_output(&node_b));

    // Set output for task B, assert that B has that output but A is unchanged.
    store.set_task_output(&node_b, output_b.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(store.task_has_output(&node_b));
    assert_eq!(store.get_task_output(&node_b), &output_b);
  }

  #[test]
  #[should_panic]
  fn test_task_has_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    store.task_has_output(&fake_node);
  }

  #[test]
  #[should_panic]
  fn test_get_task_output_panics() {
    let mut store = Store::default();
    let node = store.get_or_create_task_node(&StringConstant::new("Hello"));
    store.get_task_output(&node);
  }

  #[test]
  #[should_panic]
  fn test_set_task_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    store.set_task_output(&fake_node, "Hello".to_string());
  }


  #[test]
  fn test_dependencies() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(output_a.clone());
    let node_a = store.get_or_create_task_node(&task_a);
    let output_b = "World".to_string();
    let task_b = StringConstant::new(output_b.clone());
    let node_b = store.get_or_create_task_node(&task_b);
    let path_c = PathBuf::from("hello.txt");
    let node_c = store.get_or_create_file_node(&path_c);

    assert_eq!(store.get_dependencies_of_task(&node_a).next(), None);
    assert_eq!(store.get_dependencies_of_task(&node_b).next(), None);

    // Add file dependency from task A to file C.
    let file_dependency_a2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&node_a, &node_c, file_dependency_a2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    assert_eq!(store.get_dependencies_of_task(&node_b).next(), None);

    // Add task dependency from task B to task A.
    let task_dependency_b2a = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_b, &node_a, task_dependency_b2a.clone());
    assert_eq!(result, Ok(()));
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), None);

    // Add file dependency from task B to file C.
    let file_dependency_b2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_provide_dependency(&node_b, &node_c, file_dependency_b2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), Some(&Dependency::ProvideFile(file_dependency_b2c.clone())));
    assert_eq!(deps_of_b.get(2), None);

    // Add task dependency from task A to task B, creating a cycle.
    let task_dependency_a2b = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_a, &node_b, task_dependency_a2b);
    assert_eq!(result, Err(())); // Creates a cycle: error
  }

  #[test]
  #[should_panic]
  fn test_get_dependencies_of_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    let _ = store.get_dependencies_of_task(&fake_node);
  }

  #[test]
  #[should_panic]
  fn test_add_file_require_dependency_panics() {
    let mut fake_store = Store::default();
    let fake_file_node = fake_store.get_or_create_file_node("hello.txt");
    let fake_task_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    let dependency = FileDependency::new("hello.txt", FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&fake_task_node, &fake_file_node, dependency);
  }

  #[test]
  #[should_panic]
  fn test_add_file_provide_dependency_panics() {
    let mut fake_store = Store::default();
    let fake_file_node = fake_store.get_or_create_file_node("hello.txt");
    let fake_task_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    let dependency = FileDependency::new("hello.txt", FileStamper::Exists).unwrap();
    store.add_file_provide_dependency(&fake_task_node, &fake_file_node, dependency);
  }

  #[test]
  #[should_panic]
  fn test_add_task_require_dependency_panics() {
    let mut fake_store = Store::default();
    let output = "Hello".to_string();
    let task = StringConstant::new(&output);
    let fake_task_node = fake_store.get_or_create_task_node(&task);
    let mut store: Store<StringConstant, String> = Store::default();
    let dependency = TaskDependency::new(task, OutputStamper::Equals, output);
    let _ = store.add_task_require_dependency(&fake_task_node, &fake_task_node, dependency);
  }


  #[test]
  fn test_reset() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(output_a.clone());
    let task_a_node = store.get_or_create_task_node(&task_a);
    let output_b = "World".to_string();
    let task_b = StringConstant::new(output_b.clone());
    let task_b_node = store.get_or_create_task_node(&task_b);
    let path = PathBuf::from("hello.txt");
    let file_node = store.get_or_create_file_node(&path);

    // Set outputs for task A and B.
    store.set_task_output(&task_a_node, output_a.clone());
    assert!(store.task_has_output(&task_a_node));
    assert_eq!(store.get_task_output(&task_a_node), &output_a);
    store.set_task_output(&task_b_node, output_b.clone());
    assert!(store.task_has_output(&task_b_node));
    assert_eq!(store.get_task_output(&task_b_node), &output_b);

    // Add file dependency for task A and B.
    let file_dependency = FileDependency::new(&path, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&task_a_node, &file_node, file_dependency.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&task_a_node).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_a.get(1), None);
    store.add_file_require_dependency(&task_b_node, &file_node, file_dependency.clone());
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_b.get(1), None);

    // Reset only task A.
    store.reset_task(&task_a_node);
    // Assert that task A is reset.
    assert!(!store.task_has_output(&task_a_node));
    assert_eq!(store.get_dependencies_of_task(&task_a_node).next(), None);
    // Assert that task B is unchanged.
    assert!(store.task_has_output(&task_b_node));
    assert_eq!(store.get_task_output(&task_b_node), &output_b);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_b.get(1), None);
  }

  #[test]
  #[should_panic]
  fn test_reset_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    store.reset_task(&fake_node);
  }
}
