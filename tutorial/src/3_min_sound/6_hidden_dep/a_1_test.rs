

// Hidden dependency tests

#[test]
fn test_hidden_dependency() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in_out.txt");
  write(&file, "Hello, World!")?;

  let read = ReadFile(file.clone(), FileStamper::Modified);

  let input_file = temp_dir.path().join("in.txt");
  write(&input_file, "Hi there")?;
  let read_for_write = ReadFile(input_file.clone(), FileStamper::Modified);
  let write = WriteFile(Box::new(read_for_write.clone()), file.clone(), FileStamper::Modified);

  // Require `write` and `read`, assert they are executed because they are new.
  pie.require_then_assert_one_execute(&write)?;
  assert_eq!(read_to_string(&file)?, "Hi there");
  let output = pie.require_then_assert_one_execute(&read)?;
  assert_eq!(output.as_str(), "Hi there");

  // Although there is a hidden dependency here (`read` doesn't require `write`), we happened to have required `write`
  // and `read` in the correct order, so there is no inconsistency yet. The output of `read` is `"Hi there"` which is
  // correct.

  Ok(())
}
