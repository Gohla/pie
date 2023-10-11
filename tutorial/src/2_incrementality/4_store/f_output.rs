
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
