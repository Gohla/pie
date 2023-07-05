use std::error::Error;
use std::fs::{read_to_string, write};

use assert_matches::assert_matches;
use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use ::pie::tracker::event::Event::*;
use dev_shared::check::CheckErrorExt;
use dev_shared::fs::write_until_modified;
use dev_shared::task::{CommonOutput, CommonTask};
use dev_shared::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_exec(mut pie: TestPie<CommonTask>) -> Result<(), Box<dyn Error>> {
  let task = CommonTask::string_constant("string");
  let output = pie.require_then_assert(&task, |tracker| {
    assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
      assert_eq!(t, &task);
    });
  })?;
  assert_eq!(output.as_str(), "string");
  Ok(())
}

#[rstest]
fn test_reuse(mut pie: TestPie<CommonTask>) -> Result<(), Box<dyn Error>> {
  let task = CommonTask::string_constant("string");

  // New task: execute
  let output = pie.require_then_assert(&task, |tracker| {
    assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
      assert_eq!(t, &task);
    });
    assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
      assert_eq!(t, &task);
    });
  })?;
  assert_eq!(output.as_str(), "string");

  // Nothing changed: no execute
  pie.assert_no_execute(&task)?;

  Ok(())
}

#[rstest]
fn test_require_task(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;

  let read_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  let task = CommonTask::to_lower_case(read_task.clone());

  // Require task and observe that the tasks are executed in dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::new_string("hello world!"));

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
    assert_eq!(session.require(&task), CommonOutput::new_string("hello world!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change required file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task and observe that all tasks are re-executed in reverse dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::new_string("!dlrow olleh"));

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

  Ok(())
}

#[rstest]
fn test_require_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;
  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::new_string("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::new_string("HELLO WORLD!"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change required file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::new_string("!DLROW OLLEH"));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });

  Ok(())
}

#[rstest]
fn test_provide_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("out.txt");
  let task = CommonTask::write_constant_string_to_file("HELLO WORLD!", &path, FileStamper::Modified);

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    session.require(&task)?;
    assert_eq!(read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_execute_starts(2));
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    session.require(&task)?;
    assert_eq!(read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });

  // Change provided file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    session.require(&task)?;
    assert_eq!(read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
  });

  Ok(())
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_cycle_panics(mut pie: TestPie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.require(&CommonTask::require_self());
  });
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_panics(mut pie: TestPie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.require(&CommonTask::require_cycle_a());
  });
}


#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("out.txt");
  pie.run_in_session(|mut session| {
    let task_1 = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&task_1)?;
    assert!(session.dependency_check_errors().is_empty());
    let task_2 = CommonTask::write_constant_string_to_file("Test 2", &path, FileStamper::Modified);
    session.require(&task_2)?;
    assert!(session.dependency_check_errors().is_empty());
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("inout.txt");
  pie.run_in_session(|mut session| {
    let providing_task = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&providing_task)?;
    assert!(session.dependency_check_errors().is_empty());
    let requiring_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
    session.require(&requiring_task)?;
    assert!(session.dependency_check_errors().is_empty());
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("inout.txt");
  write(&path, "test")?;
  pie.run_in_session(|mut session| {
    let requiring_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
    session.require(&requiring_task)?;
    assert!(session.dependency_check_errors().is_empty());
    let providing_task = CommonTask::write_constant_string_to_file("Test 1", &path, FileStamper::Modified);
    session.require(&providing_task)?;
    assert!(session.dependency_check_errors().is_empty());
  });
}
