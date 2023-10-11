
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
