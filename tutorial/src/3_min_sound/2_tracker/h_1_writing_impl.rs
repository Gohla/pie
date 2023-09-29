
#[allow(dead_code)]
impl<W: Write> WritingTracker<W> {
  fn writeln(&mut self, args: std::fmt::Arguments) {
    self.write_indentation();
    let _ = writeln!(&mut self.writer, "{}", args);
  }
  fn write(&mut self, args: std::fmt::Arguments) {
    let _ = write!(&mut self.writer, "{}", args);
  }
  fn write_nl(&mut self) {
    let _ = write!(&mut self.writer, "\n");
  }

  fn indent(&mut self) {
    self.indentation = self.indentation.saturating_add(1);
  }
  fn unindent(&mut self) {
    self.indentation = self.indentation.saturating_sub(1);
  }
  fn write_indentation(&mut self) {
    for _ in 0..self.indentation {
      let _ = write!(&mut self.writer, " ");
    }
  }

  fn flush(&mut self) {
    let _ = self.writer.flush();
  }
}
