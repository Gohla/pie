use std::error::Error;
use std::fs::{create_dir_all, remove_file, write};

use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::fs::{wait_until_modified_time_changes, write_until_modified};
use dev_shared::task::CommonTask;
use dev_shared::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_modified_stamp_on_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("test.txt");

  // Modified stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  write(&path, "hello world!")?;
  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed even though file contents is the same: execute
  write_until_modified(&path, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;

  // Modified recursive stamper; should work exactly the same as modified stamper when used on a file.
  let task = CommonTask::read_string_from_file(&path, FileStamper::ModifiedRecursive);
  write(&path, "hello world!")?;
  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed even though file contents is the same: execute
  write_until_modified(&path, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}

#[rstest]
fn test_modified_stamp_on_directory(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path)?;
  let file_path_1 = dir_path.join("test1.txt");

  // Modified stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Modified);
  write(&file_path_1, "hello world!")?;
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

  // Modified recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::ModifiedRecursive);
  write(&file_path_1, "hello world!")?;
  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // File was changed and this affects the latest modified date: execute
  write_until_modified(&file_path_1, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was added and this changes directory modified time: execute
  wait_until_modified_time_changes()?;
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was removed and this changes directory modified time: execute
  wait_until_modified_time_changes()?;
  remove_file(&file_path_2)?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}

#[rstest]
fn test_hash_stamp_on_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("test.txt");

  // Hash stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Hash);
  write(&path, "hello world!")?;
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

  // Hash recursive stamper; should work exactly the same as hash stamper when used on a file.
  let task = CommonTask::read_string_from_file(&path, FileStamper::HashRecursive);
  write(&path, "hello world!")?;
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

#[rstest]
fn test_hash_stamp_on_directory(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path)?;
  let file_path_1 = dir_path.join("test1.txt");

  // Hash stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Hash);
  write(&file_path_1, "hello world!")?;
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
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was removed and this changes directory hash: execute
  remove_file(&file_path_2)?;
  pie.require_then_assert_one_execute(&task)?;

  // Hash recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::HashRecursive);
  write(&file_path_1, "hello world!")?;
  // New task: execute
  pie.require_then_assert_one_execute(&task)?;
  // Stamp unchanged: no execute
  pie.require_then_assert_no_execute(&task)?;
  // Stamp unchanged because file contents are unchanged: no execute
  write(&file_path_1, "hello world!")?;
  pie.require_then_assert_no_execute(&task)?;
  // Stamp changed because file contents are changed: execute
  write(&file_path_1, "hello world!!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was added and this changes the hash: execute
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!")?;
  pie.require_then_assert_one_execute(&task)?;
  // File was removed and this changes the hash: execute
  remove_file(&file_path_2)?;
  pie.require_then_assert_one_execute(&task)?;

  Ok(())
}
