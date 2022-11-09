use rstest::{fixture, rstest};
use tempfile::TempDir;

use crate::common::{CommonTask, Pie, ToLowerCase};

mod common;

#[fixture]
fn pie() -> Pie<CommonTask> { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_serde_roundtrip_one_task(mut pie: Pie<CommonTask>) {
  let task = CommonTask::ToLowerCase(ToLowerCase("CAPITALIZED".to_string()));
  pie.run_in_session(|mut session| {
    session.require(&task);

    let tracker = &mut session.tracker_mut().0;
    tracker.clear();
  });

  let json = serde_json::to_string(pie.store()).unwrap();
  println!("{}", json);
  let store = serde_json::from_str(&json).unwrap();
  let mut pie = pie.replace_store(store);
  pie.run_in_session(|mut session| {
    session.require(&task);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });
}
