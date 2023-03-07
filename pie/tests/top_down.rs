use std::fs;

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use ::pie::tracker::event::Event::*;
use dev_shared::check::CheckErrorExt;
use dev_shared::task::{CommonOutput, CommonTask};
use dev_shared::test::Pie;

#[fixture]
fn pie() -> Pie<CommonTask> { dev_shared::test::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { dev_shared::create_temp_dir() }


#[rstest]
fn test_exec(mut pie: Pie<CommonTask>) {
  let task = CommonTask::string_constant("string");

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::string_constant("string"));
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
  });
}

#[rstest]
fn test_reuse(mut pie: Pie<CommonTask>) {
  let task = CommonTask::string_constant("string");

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::string_constant("string"));
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
  });

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::string_constant("string"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
}

#[rstest]
fn test_require_task(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in.txt");
  fs::write(&path, "HELLO WORLD!").check();

  let read_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  let task = CommonTask::to_lower_case(read_task.clone());

  // Require task and observe that the tasks are executed in dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case_ok("hello world!"));

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
    assert!(read_task_start > task_start);

    assert!(task_end > read_task_end);
  });

  // Require task again and observe that no tasks are executed since they are not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case_ok("hello world!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task and observe that all tasks are re-executed in reverse dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case_ok("!dlrow olleh"));

    let tracker = &mut session.tracker_mut().0;

    let read_task_start = tracker.get_index_of_execute_start_of(&read_task);
    assert_matches!(read_task_start, Some(_));
    let read_task_end = tracker.get_index_of_execute_end_of(&read_task);
    assert_matches!(read_task_end, Some(_));

    let task_start = tracker.get_index_of_execute_start_of(&task);
    assert_matches!(task_start, Some(_));
    let task_end = tracker.get_index_of_execute_end_of(&task);
    assert_matches!(task_end, Some(_));
    assert!(task_end > read_task_end);
  });
}

#[rstest]
fn test_require_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in.txt");
  fs::write(&path, "HELLO WORLD!").check();
  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("!DLROW OLLEH"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });
}

#[rstest]
fn test_provide_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("out.txt");
  let task = CommonTask::write_constant_string_to_file("HELLO WORLD!", &path, FileStamper::Modified);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_execute_starts(2));
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change provided file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_cycle_panics(mut pie: Pie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.require(&CommonTask::require_self());
  });
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_panics(mut pie: Pie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.require(&CommonTask::require_cycle_a());
  });
}


#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("out.txt");
  pie.run_in_session(|mut session| {
    let task_1 = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&task_1).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let task_2 = CommonTask::write_constant_string_to_file("Test 2", &path, FileStamper::Modified);
    session.require(&task_2).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("inout.txt");
  pie.run_in_session(|mut session| {
    let providing_task = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let requiring_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("inout.txt");
  fs::write(&path, "test").check();
  pie.run_in_session(|mut session| {
    let requiring_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let providing_task = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}
