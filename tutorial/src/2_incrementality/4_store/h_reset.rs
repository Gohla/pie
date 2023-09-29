
impl<T: Task> Store<T, T::Output> {
  /// Reset task `src`, removing its output and removing all its outgoing dependencies.
  ///
  /// # Panics
  ///
  /// Panics if task `src` was not found in the dependency graph.
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(src) {
      *output = None;
    } else {
      panic!("BUG: node {:?} was not found in the dependency graph", src);
    }
    self.graph.remove_outgoing_edges_of_node(src);
  }
}
