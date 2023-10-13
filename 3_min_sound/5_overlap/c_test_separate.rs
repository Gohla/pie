
#[test]
fn test_separate_output_files() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let temp_dir = create_temp_dir()?;

  let ret = Return("Hi there");
  let output_file_1 = temp_dir.path().join("out_1.txt");
  let write_1 = WriteFile(Box::new(ret.clone()), output_file_1.clone(), FileStamper::Modified);

  let input_file = temp_dir.path().join("in.txt");
  write(&input_file, "Hello, World!")?;
  let read = ReadFile(input_file.clone(), FileStamper::Modified);
  let output_file_2 = temp_dir.path().join("out_2.txt");
  let write_2 = WriteFile(Box::new(read.clone()), output_file_2.clone(), FileStamper::Modified);

  let seq = Sequence(vec![write_1.clone(), write_2.clone()]);

  pie.require(&seq)?;
  assert_eq!(read_to_string(&output_file_1)?, "Hi there");
  assert_eq!(read_to_string(&output_file_2)?, "Hello, World!");

  write_until_modified(&input_file, "World, Hello?")?;

  // Require `write_1` to make `output_file_1` consistent.
  pie.require_then_assert_no_execute(&write_1)?;
  assert_eq!(read_to_string(&output_file_1)?, "Hi there");
  // Require `write_2` to make `output_file_2` consistent.
  pie.require_then_assert_one_execute(&write_2)?;
  assert_eq!(read_to_string(&output_file_2)?, "World, Hello?");

  Ok(())
}
