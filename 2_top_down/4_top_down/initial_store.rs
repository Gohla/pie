use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::Dependency;
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
  /// Gets the task for `node`, panicking if `node` was not found in the dependency graph or if `node` is a file.
  pub fn get_task(&self, node: &TaskNode) -> &T {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::Task { task, .. } => task,
      _ => panic!("node should be a task node")
    }
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
  /// Gets the path for `node`, panicking if `node` was not found in the dependency graph or if `node` is a task.
  #[inline]
  pub fn get_file_path(&self, node: &FileNode) -> &PathBuf {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::File(path) => path,
      _ => panic!("node should be a file node")
    }
  }


  /// Checks whether task `node` has an output. Returns `false` if `node` does not have an output , if `node` does not
  /// exist in the dependency graph, or if `node` is a file.
  #[inline]
  pub fn task_has_output(&self, node: &TaskNode) -> bool {
    if let Some(NodeData::Task { output: Some(_), .. }) = self.graph.get_node_data(node) {
      true
    } else {
      false
    }
  }
  /// Gets the output for task `node`, panicking if `node` was not found in the dependency graph, if `node` is a file,
  /// or if the task has no output.
  #[inline]
  pub fn get_task_output(&self, node: &TaskNode) -> &T::Output {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::Task { output, .. } => output.as_ref().expect("task should have an output"),
      _ => panic!("node should be a task node")
    }
  }
  /// Sets the output for task `node` to `new_output`. Does nothing if task `node` does not exist in the dependency 
  /// graph.
  #[inline]
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) {
      output.replace(new_output);
    }
  }
}
