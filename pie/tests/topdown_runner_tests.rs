use std::fs;

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::tracker::Event;
use Event::*;

use crate::common::{CheckErrorExt, CommonOutput, CommonTask, Pie, ToLowerCase};

mod common;

#[fixture]
fn pie() -> Pie<CommonTask> { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_exec(mut pie: Pie<CommonTask>) {
  let task = CommonTask::to_lower_case("CAPITALIZED");

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case("capitalized"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
      assert_eq!(t, &task);
    });
    tracker.clear();
  });
}

#[rstest]
fn test_reuse(mut pie: Pie<CommonTask>) {
  let task = CommonTask::to_lower_case("CAPITALIZED");

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case("capitalized"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
      assert_eq!(t, &task);
    });
    tracker.clear();
  });

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case("capitalized"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear()
  });
}


#[rstest]
fn test_require_task(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();

  let read_task = CommonTask::read_string_from_file(&path);
  let task = CommonTask::combine(&path);

  // Require task and observe that all three tasks are executed in dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::combine_ok("hello world!"));

    let tracker = &mut session.tracker_mut().0;

    let task_start = tracker.get_index_of_execute_start_of(&task);
    assert_matches!(task_start, Some(_));
    let task_end = tracker.get_index_of_execute_end_of(&task);
    assert_matches!(task_end, Some(_));
    assert!(task_start < task_end);

    let read_task_start = tracker.get_index_of_execute_start_of(&read_task);
    assert_matches!(read_task_start, Some(_));
    let read_task_end = tracker.get_index_of_execute_end_of(&read_task);
    assert_matches!(read_task_end, Some(_));
    assert!(read_task_start > task_start);

    let to_lowercase_task_dyn = CommonTask::to_lower_case("HELLO WORLD!");
    let to_lowercase_task_start = tracker.get_index_of_execute_start_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_start, Some(_));
    let to_lowercase_task_end = tracker.get_index_of_execute_end_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_end, Some(_));
    assert!(to_lowercase_task_start < to_lowercase_task_end);
    assert!(to_lowercase_task_start > task_start);
    assert!(to_lowercase_task_start > read_task_start);

    assert!(task_end > read_task_end);
    assert!(task_end > to_lowercase_task_end);

    tracker.clear();
  });

  // Require task again and observe that no tasks are executed since they are not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::combine_ok("hello world!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task and observe that all three tasks are re-executed in reverse dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::combine_ok("!dlrow olleh"));

    let tracker = &mut session.tracker_mut().0;

    let read_task_start = tracker.get_index_of_execute_start_of(&read_task);
    assert_matches!(read_task_start, Some(_));
    let read_task_end = tracker.get_index_of_execute_end_of(&read_task);
    assert_matches!(read_task_end, Some(_));

    // Old ToLowerCase task was not executed
    assert!(tracker.contains_no_execute_start_of(&CommonTask::ToLowerCase(ToLowerCase("HELLO WORLD!".to_string()))));

    let to_lowercase_task_dyn = CommonTask::ToLowerCase(ToLowerCase("!DLROW OLLEH".to_string()));
    let to_lowercase_task_start = tracker.get_index_of_execute_start_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_start, Some(_));
    let to_lowercase_task_end = tracker.get_index_of_execute_end_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_end, Some(_));

    let task_start = tracker.get_index_of_execute_start_of(&task);
    assert_matches!(task_start, Some(_));
    let task_end = tracker.get_index_of_execute_end_of(&task);
    assert_matches!(task_end, Some(_));

    assert!(read_task_end < to_lowercase_task_end);
    assert!(task_end > read_task_end);
    assert!(task_end > to_lowercase_task_end);

    tracker.clear();
  });

  // TODO: once stampers are implemented, only change the modification date such that ReadStringFromFile re-executes but
  //       the other tasks do not, as ReadStringFromFile still returns the same value.
}

#[rstest]
fn test_require_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();
  let task = CommonTask::read_string_from_file(&path);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("!DLROW OLLEH"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });
}

#[rstest]
fn test_provide_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  let task = CommonTask::write_string_to_file("HELLO WORLD!", &path);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change provided file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_cycle_panics(mut pie: Pie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.require(&CommonTask::require_self());
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  pie.run_in_session(|mut session| {
    let task_1 = CommonTask::write_string_to_file("Test 1", &path);
    session.require(&task_1).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let task_2 = CommonTask::write_string_to_file("Test 2", &path);
    session.require(&task_2).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  pie.run_in_session(|mut session| {
    let providing_task = CommonTask::write_string_to_file("Test 1", &path);
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let requiring_task = CommonTask::read_string_from_file(&path);
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "test").check();
  pie.run_in_session(|mut session| {
    let requiring_task = CommonTask::read_string_from_file(&path);
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let providing_task = CommonTask::write_string_to_file("Test 1", &path);
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}
