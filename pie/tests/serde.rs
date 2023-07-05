use std::error::Error;
use std::fs;

use ron::{Deserializer, Serializer};
use ron::ser::PrettyConfig;
use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::task::*;
use dev_shared::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_serde_round_trip_one_task(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!")?;
  let task = ToLowerCase::new(ReadStringFromFile::new(&path, FileStamper::Modified));
  pie.require(&task)?;

  let mut buffer = Vec::new();
  let mut serializer = Serializer::new(&mut buffer, Some(PrettyConfig::default()))?;
  pie.serialize(&mut serializer)?;
  println!("{}", String::from_utf8(buffer.clone())?);

  let mut deserializer = Deserializer::from_bytes(&buffer)?;
  let mut pie = pie.deserialize(&mut deserializer)?;

  // After serialize-deserialize round-trip, no task should be executed because nothing changed.
  pie.require_then_assert_no_execute(&task)?;

  Ok(())
}
