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
