
/// Testing tasks enumeration.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TestTask {
  Return(&'static str),
}
impl Task for TestTask {
  type Output = Result<TestOutput, ErrorKind>;
  fn execute<C: Context<Self>>(&self, _context: &mut C) -> Self::Output {
    match self {
      TestTask::Return(string) => Ok(string.to_string().into()),
    }
  }
}

/// [`TestTask`] output enumeration.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TestOutput {
  String(String),
}
impl From<String> for TestOutput {
  fn from(value: String) -> Self { Self::String(value) }
}
impl TestOutput {
  pub fn as_str(&self) -> &str {
    match self {
      Self::String(s) => &s,
    }
  }
}
