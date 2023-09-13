
/// [`Tracker`] that forwards build events to 2 trackers.
#[derive(Copy, Clone, Debug)]
pub struct CompositeTracker<A1, A2>(pub A1, pub A2);
impl<T: Task, A1: Tracker<T>, A2: Tracker<T>> Tracker<T> for CompositeTracker<A1, A2> {
  fn build_start(&mut self) {
    self.0.build_start();
    self.1.build_start();
  }
  fn build_end(&mut self) {
    self.0.build_end();
    self.1.build_end();
  }

  fn required_file(&mut self, path: &Path, stamper: &FileStamper, stamp: &FileStamp) {
    self.0.required_file(path, stamper, stamp);
    self.1.required_file(path, stamper, stamp);
  }
  fn require_task(&mut self, task: &T, stamper: &OutputStamper) {
    self.0.require_task(task, stamper);
    self.1.require_task(task, stamper);
  }
  fn required_task(&mut self, task: &T, output: &T::Output, stamper: &OutputStamper, stamp: &OutputStamp<T::Output>, was_executed: bool) {
    self.0.required_task(task, output, stamper, stamp, was_executed);
    self.1.required_task(task, output, stamper, stamp, was_executed);
  }

  fn execute(&mut self, task: &T) {
    self.0.execute(task);
    self.1.execute(task);
  }
  fn executed(&mut self, task: &T, output: &T::Output) {
    self.0.executed(task, output);
    self.1.executed(task, output);
  }
}
