
impl<W: Write, T: Task> Tracker<T> for WritingTracker<W> {
  fn build_start(&mut self) {
    self.indentation = 0;
  }
  fn build_end(&mut self) {
    self.writeln(format_args!("🏁"));
    self.flush();
  }

  fn require_file_end(&mut self, dependency: &FileDependency) {
    self.writeln(format_args!("- {}", dependency.path().display()));
  }
  fn require_task_start(&mut self, task: &T, _stamper: &OutputStamper) {
    self.writeln(format_args!("→ {:?}", task));
    self.indent();
    self.flush();
  }
  fn require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, output: &T::Output, _was_executed: bool) {
    self.unindent();
    self.writeln(format_args!("← {:?}", output));
    self.flush();
  }

  fn check_dependency_start(&mut self, dependency: &Dependency<T, T::Output>) {
    match dependency {
      Dependency::RequireTask(d) => {
        self.writeln(format_args!("? {:?}", d.task()));
        self.indent();
        self.flush();
      },
      _ => {},
    }
  }
  fn check_dependency_end(
    &mut self,
    dependency: &Dependency<T, T::Output>,
    inconsistency: Result<Option<&Inconsistency<T::Output>>, &io::Error>
  ) {
    match dependency {
      Dependency::RequireFile(d) => {
        match inconsistency {
          Err(e) => self.writeln(format_args!("✗ {} (err: {:?})", d.path().display(), e)),
          Ok(Some(Inconsistency::File(s))) =>
            self.writeln(format_args!("✗ {} (old: {:?} ≠ new: {:?})", d.path().display(), d.stamp(), s)),
          Ok(None) => self.writeln(format_args!("✓ {}", d.path().display())),
          _ => {}, // Other variants cannot occur.
        }
      },
      Dependency::RequireTask(d) => {
        self.unindent();
        match inconsistency {
          Ok(Some(Inconsistency::Task(s))) =>
            self.writeln(format_args!("✗ {:?} (old: {:?} ≠ new: {:?})", d.task(), d.stamp(), s)),
          Ok(None) => self.writeln(format_args!("✓ {:?}", d.task())),
          _ => {}, // Other variants cannot occur.
        }
      }
    }
    self.flush()
  }

  fn execute_start(&mut self, task: &T) {
    self.writeln(format_args!("▶ {:?}", task));
    self.indent();
    self.flush();
  }
  fn execute_end(&mut self, _task: &T, output: &T::Output) {
    self.unindent();
    self.writeln(format_args!("◀ {:?}", output));
    self.flush();
  }
}
