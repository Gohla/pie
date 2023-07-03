use std::hash::BuildHasher;
use std::io::Stdout;

use rstest::fixture;
use tempfile::TempDir;

use ::pie::Pie;
use ::pie::Session;
use ::pie::Task;
use ::pie::tracker::CompositeTracker;
use ::pie::tracker::event::EventTracker;
use ::pie::tracker::writing::WritingTracker;

/// Testing tracker composed of an [`EventTracker`] for testing and stdout [`WritingTracker`] for debugging.
pub type TestTracker<T> = CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>;

/// Testing [`Pie`] using [`TestTracker`].
pub type TestPie<T> = Pie<T, <T as Task>::Output, TestTracker<T>>;

#[inline]
pub fn create_test_pie<T: Task>() -> TestPie<T> {
  let tracker = CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer());
  TestPie::with_tracker(tracker)
}

// Testing fixtures

#[fixture]
#[inline]
pub fn pie<T: Task>() -> TestPie<T> {
  create_test_pie()
}

#[fixture]
#[inline]
pub fn temp_dir() -> TempDir {
  crate::fs::create_temp_dir()
}

// Testing utilities

pub trait TestPieExt<T: Task, H: BuildHasher + Default> {
  /// Runs `run_func` inside a new session, asserts that there are no dependency check errors, then runs `test_func` on 
  /// the event tracker for testing assertions.
  fn test_in_session<R, E>(
    &mut self,
    run_func: impl FnOnce(&mut Session<T, T::Output, TestTracker<T>, H>) -> Result<R, E>,
    test_func: impl FnOnce(&EventTracker<T>) -> Result<(), E>,
  ) -> Result<R, E>;

  /// Requires `task` inside a new session, asserts that there are no dependency check errors, then runs `test_func` on 
  /// the event tracker for testing assertions.
  fn require_then_test<R, E, O: Into<Result<R, E>>>(
    &mut self,
    task: &T,
    test_func: impl FnOnce(&EventTracker<T>) -> Result<(), E>,
  ) -> Result<R, E> where T: Task<Output=O> {
    self.test_in_session(|s| s.require(task).into(), test_func)
  }

  /// Require `task` in a new session, assert that it is executed.
  fn assert_one_execute<R, E, O: Into<Result<R, E>>>(&mut self, task: &T) -> Result<R, E> where T: Task<Output=O> {
    self.require_then_test(task, |t| {
      assert!(t.contains_one_execute_start_of(task), "expected execution of task {:?}, but it was not executed", task);
      Ok(())
    })
  }

  /// Require `task` in a new session, assert that it is not executed.
  fn assert_no_execute<R, E, O: Into<Result<R, E>>>(&mut self, task: &T) -> Result<R, E> where T: Task<Output=O> {
    self.require_then_test(task, |t| {
      assert!(t.contains_no_execute_start_of(task), "expected no execution of task {:?}, but it was executed", task);
      Ok(())
    })
  }
}

impl<T: Task, H: BuildHasher + Default> TestPieExt<T, H> for Pie<T, T::Output, TestTracker<T>, H> {
  fn test_in_session<R, E>(
    &mut self,
    run_func: impl FnOnce(&mut Session<T, T::Output, TestTracker<T>, H>) -> Result<R, E>,
    test_func: impl FnOnce(&EventTracker<T>) -> Result<(), E>
  ) -> Result<R, E> {
    let mut session = self.new_session();
    let output = run_func(&mut session)?;
    assert!(session.dependency_check_errors().is_empty());
    test_func(&self.tracker().0)?;
    Ok(output)
  }
}
