use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

/// Stores files and tasks, and their dependencies, in a DAG (directed acyclic graph). Provides operations to mutate
/// and query this graph.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "T: serde::Serialize + Task, O: serde::Serialize, H: BuildHasher + Default")))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "T: serde::Deserialize<'de> + Task, O: serde::Deserialize<'de>, H: BuildHasher + Default")))]
pub struct Store<T, O, H> {
  graph: DAG<NodeData<T, O>, Option<Dependency<T, O>>, H>,
  file_to_node: HashMap<PathBuf, FileNode, H>,
  task_to_node: HashMap<T, TaskNode, H>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FileNode(Node);

impl Borrow<Node> for &FileNode {
  fn borrow(&self) -> &Node { &self.0 }
}


/// Newtype for task `Node`s.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TaskNode(Node);

impl Borrow<Node> for &TaskNode {
  fn borrow(&self) -> &Node { &self.0 }
}

impl<T: Task, H: BuildHasher + Default> Default for Store<T, T::Output, H> {
  #[inline]
  fn default() -> Self {
    Self {
      graph: DAG::default(),
      file_to_node: HashMap::default(),
      task_to_node: HashMap::default(),
    }
  }
}

impl<T: Task, H: BuildHasher + Default> Store<T, T::Output, H> {
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


  /// Get all dependencies of task `src`. 
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` was not found in the dependency graph.
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Option<Dependency<T, T::Output>>> + 'a {
    debug_assert!(self.graph.contains_node(src), "BUG: node {:?} was not found in the dependency graph", src);
    self.graph.get_outgoing_edge_data(src)
  }
  /// Add a file require `dependency` from task `src` to file `dst`.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` was not found in the dependency graph, or if a cycle is created by adding this dependency.
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    match self.graph.add_edge(src, dst, Some(Dependency::RequireFile(dependency))) {
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
    match self.graph.add_edge(src, dst, Some(Dependency::ProvideFile(dependency))) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => panic!("BUG: cycle detected when adding file dependency from {:?} to {:?}", src, dst),
      _ => {},
    }
  }
  /// Reserve a task require dependency from task `src` to task `dst`. This reservation is required because the 
  /// dependency from `src` to `dst` should already exist for validation purposes (cycles, hidden dependencies, etc.), 
  /// but we do not yet have the output of task `dst` so we cannot fully create the dependency.
  ///
  /// # Errors
  ///
  /// Returns `Err(())` if adding this dependency to the graph creates a cycle.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph.
  #[inline]
  pub fn reserve_task_require_dependency(&mut self, src: &TaskNode, dst: &TaskNode) -> Result<(), ()> {
    match self.graph.add_edge(src, dst, None) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => Err(()),
      _ => Ok(()),
    }
  }
  /// Update the reserved task require dependency from task `src` to `dst` to `dependency`. 
  ///
  /// # Panics
  ///
  /// Panics if the dependency was not reserved before.
  #[inline]
  pub fn update_reserved_task_require_dependency(&mut self, src: &TaskNode, dst: &TaskNode, dependency: TaskDependency<T, T::Output>) {
    let Some(edge_data) = self.graph.get_edge_data_mut(src, dst) else {
      panic!("BUG: no reserved task dependency found between source node {:?} and desination node {:?}", src, dst)
    };
    edge_data.replace(Dependency::RequireTask(dependency));
  }


  /// Reset task `src`, removing its output and removing all its dependencies.
  ///
  /// # Panics
  ///
  /// Panics if task `src` was not found in the dependency graph.
  #[inline]
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(src) {
      *output = None;
    } else {
      panic!("BUG: node {:?} was not found in the dependency graph", src);
    }
    self.graph.remove_edges_of_node(src);
  }


  /// Checks whether there is a direct or transitive dependency from task `src` to task `dst`. Returns false when either
  /// node was not found in the dependency graph.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, src: &TaskNode, dst: &TaskNode) -> bool {
    self.graph.contains_transitive_edge(src, dst)
  }
  /// Compare task `node_a` and  task `node_b`, topographically.
  ///
  /// # Panics
  ///
  /// Panics if task `node_a` or `node_b` were not found in the dependency graph.
  #[inline]
  pub fn topologically_compare(&self, node_a: &TaskNode, node_b: &TaskNode) -> std::cmp::Ordering {
    self.graph.topo_cmp(node_a, node_b)
  }


  /// Get all file nodes that are provided by task `src`.
  #[inline]
  pub fn get_provided_files<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=FileNode> + '_ {
    self.graph.get_outgoing_edges(src)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(FileNode(*n)) } else { None }))
  }
  /// Get all task nodes and corresponding dependencies that require task `dst`.
  #[inline]
  pub fn get_tasks_requiring_task<'a>(&'a self, dst: &'a TaskNode) -> impl Iterator<Item=(TaskNode, &TaskDependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_task_dependency().map(|d| (TaskNode(*n), d))))
  }
  /// Get all task nodes and corresponding dependencies that require file `dst`.
  #[inline]
  pub fn get_tasks_requiring_file<'a>(&'a self, dst: &'a FileNode) -> impl Iterator<Item=(TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_require_file_dependency().map(|d| (TaskNode(*n), d))))
  }
  /// Get the task node that provides file `dst`, or `None` if there is none.
  #[inline]
  pub fn get_task_providing_file(&self, dst: &FileNode) -> Option<TaskNode> {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(TaskNode(*n)) } else { None })).next()
  }
  /// Get all task nodes and corresponding dependencies that require or provide file `dst`.
  #[inline]
  pub fn get_tasks_requiring_or_providing_file<'a>(&'a self, dst: &'a FileNode, provide: bool) -> impl Iterator<Item=(TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(move |(n, d)| d.as_ref().and_then(move |d| d.as_require_or_provide_file_dependency(provide).map(|d| (TaskNode(*n), d))))
  }
}
