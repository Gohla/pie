use rstest::{fixture, rstest};
use serde_json::Serializer;
use tempfile::TempDir;

use crate::common::Pie;

mod common;

#[fixture]
fn pie() -> Pie { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_serialize_empty(pie: Pie) {
  let mut writer = Vec::with_capacity(128);
  let mut serializer = Serializer::new(&mut writer);
  pie.serialize(&mut serializer).unwrap();
  let string = String::from_utf8(writer).unwrap();
  println!("{}", string);
}
