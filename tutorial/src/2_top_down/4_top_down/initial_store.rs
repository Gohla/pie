use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

pub type TaskNode = Node;
pub type FileNode = Node;

pub struct Store<T, O> {
  graph: DAG<NodeData<T, O>, Option<Dependency<T, O>>>,
  task_to_node: HashMap<T, TaskNode>,
  file_to_node: HashMap<PathBuf, FileNode>,
}

#[derive(Debug)]
pub enum NodeData<T, O> {
  Task {
    task: T,
    output: Option<O>,
  },
  File(PathBuf),
}

impl<T: Task> Default for Store<T, T::Output> {
  fn default() -> Self {
    Self {
      graph: DAG::with_default_hasher(),
      task_to_node: HashMap::default(),
      file_to_node: HashMap::default(),
    }
  }
}

impl<T: Task> Store<T, T::Output> {
  /// Gets the node for `task`, or creates a node by adding it to the dependency graph.
  pub fn get_or_create_task_node(&mut self, task: &T) -> TaskNode {
    if let Some(node) = self.task_to_node.get(task) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Task {
        task: task.clone(),
        output: None,
      });
      self.task_to_node.insert(task.clone(), node);
      node
    }
  }
  /// Gets the task for `node`. Panics if `node` was not found in the dependency graph or if `node` is a file.
  pub fn get_task(&self, node: &TaskNode) -> &T {
    let Some(NodeData::Task { task, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node does not exist in dependency graph or is not a task node")
    };
    task
  }


  /// Gets the file node for `path`, or creates a node by adding it to the dependency graph.
  pub fn get_or_create_file_node(&mut self, path: impl AsRef<Path>) -> FileNode {
    let path = path.as_ref();
    if let Some(file_node) = self.file_to_node.get(path) {
      *file_node
    } else {
      let node = self.graph.add_node(NodeData::File(path.to_path_buf()));
      self.file_to_node.insert(path.to_path_buf(), node);
      node
    }
  }
  /// Gets the path for `node`. Panics if `node` was not found in the dependency graph or if `node` is a task.
  pub fn get_file_path(&self, node: &FileNode) -> &PathBuf {
    let Some(NodeData::File(path)) = self.graph.get_node_data(node) else {
      panic!("BUG: node does not exist in dependency graph or is not a file node")
    };
    path
  }


  /// Checks whether task `node` has an output. Returns `false` if `node` does not have an output. Panics if task `node` 
  /// was not found in the dependency graph or if `node` is a file.
  pub fn task_has_output(&self, node: &TaskNode) -> bool {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node does not exist in dependency graph or is not a task node")
    };
    output.is_some()
  }
  /// Gets the output for task `node`. Panics if `node` was not found in the dependency graph, if `node` is a file,
  /// or if the task has no output.
  pub fn get_task_output(&self, node: &TaskNode) -> &T::Output {
    let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: node does not exist in dependency graph, is not a task node, or does not have an output");
    };
    output
  }
  /// Sets the output for task `node` to `new_output`. Panics if task `node` was not found in the dependency graph or if 
  /// `node` is a file.
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: T::Output) {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) else {
      panic!("BUG: node does not exist in dependency graph or is not a task node")
    };
    output.replace(new_output);
  }


  /// Get all dependencies of task `src`.
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Option<Dependency<T, T::Output>>> + 'a {
    self.graph.get_outgoing_edge_data(src)
  }
  /// Add a file require dependency from task `src` to file `dst`.
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    // Ignore Result: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    let _ = self.graph.add_edge(src, dst, Some(Dependency::RequireFile(dependency)));
  }
  /// Reserve a task require dependency from task `src` to task `dst`. Returns an `Err` if this dependency creates a 
  /// cycle. This reservation is required because the dependency from `src` to `dst` should already exist for 
  /// cycle checking, but we do not yet have the output of task `dst` so we cannot fully create the dependency.
  pub fn reserve_task_require_dependency(&mut self, src: &TaskNode, dst: &Node) -> Result<(), pie_graph::Error> {
    self.graph.add_edge(src, dst, None)?;
    Ok(())
  }
  /// Update the reserved task require dependency from task `src` to `dst` to `dependency`. Panics if the dependency was
  /// not reserved before.
  pub fn update_reserved_task_require_dependency(&mut self, src: &TaskNode, dst: &Node, dependency: TaskDependency<T, T::Output>) {
    self.graph.get_edge_data_mut(src, dst).unwrap().replace(Dependency::RequireTask(dependency));
  }

  /// Reset task `src`, removing its output and removing all its dependencies.
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(data) = self.graph.get_node_data_mut(src) {
      match data {
        NodeData::Task { output, .. } => *output = None,
        _ => {}
      }
    }
    self.graph.remove_edges_of_node(src);
  }
}
