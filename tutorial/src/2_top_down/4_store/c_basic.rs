use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

pub type TaskNode = Node;
pub type FileNode = Node;

pub struct Store<T, O> {
  graph: DAG<NodeData<T, O>, Option<Dependency<T, O>>>,
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
    }
  }
}
