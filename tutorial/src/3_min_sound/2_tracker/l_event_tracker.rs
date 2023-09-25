
impl<T: Task> Tracker<T> for EventTracker<T, T::Output> {
  fn build_start(&mut self) {
    self.events.clear();
  }

  fn required_file(&mut self, path: &Path, stamper: &FileStamper, stamp: &FileStamp) {
    self.events.push(Event::RequiredFile { path: path.to_path_buf(), stamper: *stamper, stamp: *stamp });
  }
  fn require_task(&mut self, task: &T, stamper: &OutputStamper) {
    self.events.push(Event::RequireTask { task: task.clone(), stamper: stamper.clone() });
  }
  fn required_task(&mut self, task: &T, output: &T::Output, stamper: &OutputStamper, stamp: &OutputStamp<T::Output>, was_executed: bool) {
    self.events.push(Event::RequiredTask { task: task.clone(), output: output.clone(), stamper: *stamper, stamp: stamp.clone(), was_executed });
  }

  fn execute(&mut self, task: &T) {
    self.events.push(Event::Execute { task: task.clone() });
  }
  fn executed(&mut self, task: &T, output: &T::Output) {
    self.events.push(Event::Executed { task: task.clone(), output: output.clone() });
  }
}
