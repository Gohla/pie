use std::hash::BuildHasher;
use std::io::Stdout;
use std::path::PathBuf;

use rstest::fixture;
use tempfile::TempDir;

use ::pie::Pie;
use ::pie::Session;
use ::pie::Task;
use ::pie::tracker::CompositeTracker;
use ::pie::tracker::event::EventTracker;
use ::pie::tracker::writing::WritingTracker;

// Testing fixtures

#[fixture]
#[inline]
pub fn pie<T: Task>() -> TestPie<T> {
  create_test_pie()
}

#[fixture]
#[inline]
pub fn temp_dir() -> TempDir {
  dev_shared::create_temp_dir().expect("failed to create temporary directory")
}


// Testing utilities

/// Testing tracker composed of an [`EventTracker`] for testing and stdout [`WritingTracker`] for debugging.
pub type TestTracker<T> = CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>;

/// Testing [`Pie`] using [`TestTracker`].
pub type TestPie<T> = Pie<T, <T as Task>::Output, TestTracker<T>>;

#[inline]
pub fn create_test_pie<T: Task>() -> TestPie<T> {
  let tracker = CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer());
  TestPie::with_tracker(tracker)
}

/// Testing extensions for [`TestPie`].
pub trait TestPieExt<T: Task, H: BuildHasher + Default> {
  /// Runs `run_func` in a new session, asserts that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn assert_in_session<R>(
    &mut self,
    run_func: impl FnOnce(&mut Session<T, T::Output, TestTracker<T>, H>) -> R,
    test_assert_func: impl FnOnce(&EventTracker<T>),
  ) -> R;

  /// Require `task` in a new session, asserts that there are no dependency check errors.
  #[inline]
  fn require(&mut self, task: &T) -> T::Output {
    self.assert_in_session(|s| s.require(task), |_| {})
  }
  /// Require `task` in a new session, asserts that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  #[inline]
  fn require_then_assert(
    &mut self,
    task: &T,
    test_assert_func: impl FnOnce(&EventTracker<T>),
  ) -> T::Output {
    self.assert_in_session(|s| s.require(task), test_assert_func)
  }
  /// Require `task` in a new session, then assert that it is not executed.
  #[inline]
  fn require_then_assert_no_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(!t.any_execute_of(task), "expected no execution of task {:?}, but it was executed", task),
    )
  }
  /// Require `task` in a new session, then assert that it is executed exactly once.
  #[inline]
  fn require_then_assert_one_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(t.one_execute_of(task), "expected one execution of task {:?}, but it was not executed, or was executed more than once", task),
    )
  }

  /// Make up-to-date all tasks affected by `changed_files` in a new session, asserts that there are no dependency check
  /// errors, then runs `test_assert_func` on the event tracker for test assertion purposes.
  #[inline]
  fn update_affected_by_then_assert<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(
    &mut self,
    changed_files: I,
    test_assert_func: impl FnOnce(&EventTracker<T>),
  ) {
    self.assert_in_session(|s| s.update_affected_by(changed_files), test_assert_func)
  }
}

impl<T: Task, H: BuildHasher + Default> TestPieExt<T, H> for Pie<T, T::Output, TestTracker<T>, H> {
  #[inline]
  fn assert_in_session<R>(
    &mut self,
    run_func: impl FnOnce(&mut Session<T, T::Output, TestTracker<T>, H>) -> R,
    test_func: impl FnOnce(&EventTracker<T>),
  ) -> R {
    let mut session = self.new_session();
    let output = run_func(&mut session);
    assert!(session.dependency_check_errors().is_empty());
    test_func(&self.tracker().0);
    output
  }
}
