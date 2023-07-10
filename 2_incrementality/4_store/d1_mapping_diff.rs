use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

/// Stores files and tasks, and their dependencies, in a DAG (directed acyclic graph). Provides operations to mutate
/// and query this graph.
pub struct Store<T, O> {
  graph: DAG<NodeData<T, O>, Dependency<T, O>>,
  file_to_node: HashMap<PathBuf, Node>,
  task_to_node: HashMap<T, Node>,
}

#[derive(Debug)]
enum NodeData<T, O> {
  File(PathBuf),
  Task {
    task: T,
    output: Option<O>,
  },
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
