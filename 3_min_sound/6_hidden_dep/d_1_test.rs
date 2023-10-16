

// Hidden dependency tests

#[should_panic(expected = "Hidden dependency")]
#[test]
fn test_require_hidden_dependency_panics() {
  fn run() -> Result<(), io::Error> {
    let mut pie = test_pie();
    let temp_dir = create_temp_dir()?;

    let file = temp_dir.path().join("in_out.txt");
    write(&file, "Hello, World!")?;

    let read = ReadFile(file.clone(), FileStamper::Modified);
    let write = WriteFile(Box::new(Return("Hi there")), file.clone(), FileStamper::Modified);

    pie.require_then_assert_one_execute(&write)?;
    pie.require_then_assert_one_execute(&read)?;

    Ok(())
  }
  run().unwrap();
}
