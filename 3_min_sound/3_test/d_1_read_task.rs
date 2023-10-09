use std::io::{BufWriter, ErrorKind, Read, Stdout};
use std::path::PathBuf;

use pie::{Context, Pie, Task};
use pie::stamp::FileStamper;
use pie::tracker::CompositeTracker;
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;

/// Testing tracker composed of an [`EventTracker`] for testing and stdout [`WritingTracker`] for debugging.
pub type TestTracker<T> = CompositeTracker<EventTracker<T, <T as Task>::Output>, WritingTracker<BufWriter<Stdout>>>;
pub fn test_tracker<T: Task>() -> TestTracker<T> {
  CompositeTracker(EventTracker::default(), WritingTracker::with_stdout())
}

/// Testing [`Pie`] using [`TestTracker`].
pub type TestPie<T> = Pie<T, <T as Task>::Output, TestTracker<T>>;
pub fn test_pie<T: Task>() -> TestPie<T> {
  TestPie::with_tracker(test_tracker())
}

/// Testing extensions for [`TestPie`].
pub trait TestPieExt<T: Task> {
  /// Require `task` in a new session, assert that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn require_then_assert(
    &mut self,
    task: &T,
    test_assert_func: impl FnOnce(&EventTracker<T, T::Output>),
  ) -> T::Output;

  /// Require `task` in a new session, asserts that there are no dependency check errors.
  fn require(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |_| {})
  }

  /// Require `task` in a new session, then assert that it is not executed.
  fn require_then_assert_no_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(!t.any_execute_of(task), "expected no execution of task {:?}, but it was executed", task),
    )
  }
  /// Require `task` in a new session, then assert that it is executed exactly once.
  fn require_then_assert_one_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(t.one_execute_of(task), "expected one execution of task {:?}, but it was not executed, or was executed more than once", task),
    )
  }
}
impl<T: Task> TestPieExt<T> for TestPie<T> {
  fn require_then_assert(&mut self, task: &T, test_assert_func: impl FnOnce(&EventTracker<T, T::Output>)) -> T::Output {
    let mut session = self.new_session();
    let output = session.require(task);
    assert!(session.dependency_check_errors().is_empty(), "expected no dependency checking errors, but there are \
    dependency checking errors: {:?}", session.dependency_check_errors());
    test_assert_func(&self.tracker().0);
    output
  }
}

/// Testing tasks enumeration.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TestTask {
  Return(&'static str),
  ReadFile(PathBuf, FileStamper),
}
impl Task for TestTask {
  type Output = Result<TestOutput, ErrorKind>;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      TestTask::Return(string) => Ok(string.to_string().into()),
      TestTask::ReadFile(path, stamper) => {
        let mut string = String::new();
        if let Some(mut file) = context.require_file_with_stamper(path, *stamper).map_err(|e| e.kind())? {
          file.read_to_string(&mut string).map_err(|e| e.kind())?;
        }
        Ok(string.into())
      }
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
