use std::fs;

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::stamp::FileStamper;

use crate::common::{CheckErrorExt, CommonOutput, CommonTask, Pie};

mod common;

#[fixture]
fn pie() -> Pie<CommonTask> { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_nothing_affected(mut pie: Pie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.update_affected_by(&[]);
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear()
  });
}

#[rstest]
fn test_directly_affected_task(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();

  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    tracker.clear();
  });

  fs::write(&path, "hello world!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[path]);
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_end_of_with(&task, &CommonOutput::read_string_from_file_ok("hello world!")));
    tracker.clear()
  });
}

#[rstest]
fn test_indirectly_affected_task(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();

  let read_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  let task = CommonTask::combine(&path, FileStamper::Modified);

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::combine_ok("hello world!"));
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    tracker.clear();
  });

  fs::write(&path, "HELLO WORLD!!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[path]);
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    let read_task_end = tracker.get_index_of_execute_end_of_with(&read_task, &CommonOutput::read_string_from_file_ok("HELLO WORLD!!"));
    assert_matches!(read_task_end, Some(_));
    let task_end = tracker.get_index_of_execute_end_of_with(&task, &CommonOutput::combine_ok("hello world!!"));
    assert_matches!(task_end, Some(_));
    assert!(task_end > read_task_end);
    tracker.clear()
  });
}
