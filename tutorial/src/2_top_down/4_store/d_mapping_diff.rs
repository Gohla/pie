use std::collections::HashMap;
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
enum NodeData<T, O> {
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
