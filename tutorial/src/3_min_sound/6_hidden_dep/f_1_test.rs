
#[test]
fn test_non_hidden_dependency() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in_out.txt");
  write(&file, "Hello, World!")?;

  let input_file = temp_dir.path().join("in.txt");
  write(&input_file, "Hi There!")?;
  let read_input = ReadFile(input_file.clone(), FileStamper::Modified, None);
  let write = WriteFile(Box::new(read_input.clone()), file.clone(), FileStamper::Modified);
  let read = ReadFile(file.clone(), FileStamper::Modified, Some(Box::new(write.clone())));

  // Require `read`, which requires `write` to update the provided file. All tasks are executed because they are new.
  let output = pie.require_then_assert(&read, |tracker| {
    assert!(tracker.one_execute_of(&read));
    assert!(tracker.one_execute_of(&write));
    assert!(tracker.one_execute_of(&read_input));
  })?;
  // `read` should output what `write` wrote, which is what `read_input` read from `input_file`.
  assert_eq!(output.as_str(), "Hi There!");

  Ok(())
}
