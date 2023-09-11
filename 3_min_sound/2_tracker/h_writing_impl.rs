
impl<W: Write, T: Task> Tracker<T> for WritingTracker<W> {
  fn required_task(&mut self, task: &T, _output: &T::Output, _stamper: &OutputStamper, _stamp: &OutputStamp<T::Output>, was_executed: bool) {
    if !was_executed {
      self.writeln(format_args!("✓ {:?}", task));
      self.flush();
    }
  }

  fn execute(&mut self, task: &T) {
    self.writeln(format_args!("→ {:?}", task));
    self.indent();
    self.flush();
  }
  fn executed(&mut self, _task: &T, output: &T::Output) {
    self.unindent();
    self.writeln(format_args!("← {:?}", output));
    self.flush();
  }
}

#[allow(dead_code)]
impl<W: Write> WritingTracker<W> {
  fn writeln(&mut self, args: std::fmt::Arguments) {
    self.write_indentation();
    writeln!(&mut self.writer, "{}", args).ok();
  }
  fn write(&mut self, args: std::fmt::Arguments) {
    write!(&mut self.writer, "{}", args).ok();
  }
  fn write_nl(&mut self) {
    write!(&mut self.writer, "\n").ok();
  }

  fn indent(&mut self) {
    self.indentation = self.indentation.saturating_add(1);
  }
  fn unindent(&mut self) {
    self.indentation = self.indentation.saturating_sub(1);
  }
  fn write_indentation(&mut self) {
    for _ in 0..self.indentation {
      write!(&mut self.writer, " ").ok();
    }
  }

  fn flush(&mut self) {
    self.writer.flush().ok();
  }
}
