use std::error::Error;
use std::fs::{create_dir_all, remove_file, write};

use dev_ext::task::*;
use dev_util::{create_temp_dir, wait_until_modified_time_changes, write_until_modified};
use pie::resource::file::{HashChecker, ModifiedChecker};

use crate::util::{new_test_pie, TestPieExt};

mod util;

#[test]
fn test_modified_checker_on_file() -> Result<(), Box<dyn Error>> {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let path = temp_dir.path().join("test.txt");
  write(&path, "hello world!")?;

  let task = ReadFile::new(&path).with_checker(ModifiedChecker);

  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed even though file contents is the same: execute
  write_until_modified(&path, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}

#[test]
fn test_modified_checker_on_directory() -> Result<(), Box<dyn Error>> {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path)?;
  let file_path_1 = dir_path.join("test_1.txt");
  write(&file_path_1, "hello world!")?;

  let task = ListDirectory::with_checker(&dir_path, ModifiedChecker);

  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // File was changed but this does not affect directory modified time: no execute
  write_until_modified(&file_path_1, "hello world!")?;
  pie.require_then_assert_no_execute(&task)?;
  // File was added and this changes directory modified time: execute
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was removed and this changes directory modified time: execute
  wait_until_modified_time_changes()?;
  remove_file(&file_path_2)?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}

#[test]
fn test_hash_checker_on_file() -> Result<(), Box<dyn Error>> {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let path = temp_dir.path().join("test.txt");
  write(&path, "hello world!")?;

  let task = ReadFile::new(&path).with_checker(HashChecker);

  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp unchanged because file contents are unchanged: no execute
  write(&path, "hello world!")?;
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed because file contents are changed: execute
  write(&path, "hello world!!")?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}

#[test]
fn test_hash_checker_on_directory() -> Result<(), Box<dyn Error>> {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path)?;
  let file_path_1 = dir_path.join("test_1.txt");
  write(&file_path_1, "hello world!")?;

  let task = ListDirectory::with_checker(&dir_path, HashChecker);

  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp unchanged because file contents are unchanged: no execute
  write(&file_path_1, "hello world!")?;
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed because file contents are changed, but does not affect directory hash: no execute
  write(&file_path_1, "hello world!!")?;
  pie.require_then_assert_no_execute(&task)?;
  // File was added and this changes directory hash: execute
  let file_path_2 = dir_path.join("test_2.txt");
  write(&file_path_2, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was removed and this changes directory hash: execute
  remove_file(&file_path_2)?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}
