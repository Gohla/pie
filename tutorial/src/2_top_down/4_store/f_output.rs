impl<T: Task> Store<T, T::Output> {
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
}
