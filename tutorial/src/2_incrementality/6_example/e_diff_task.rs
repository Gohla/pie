
  let input_file_b = temp_dir.path().join("input_b.txt");
  write_until_modified(&input_file_b, "Test")?;
  let read_task_b_modified = ReadStringFromFile::new(&input_file_b, FileStamper::Modified);
  let read_task_b_exists = ReadStringFromFile::new(&input_file_b, FileStamper::Exists);
  println!("\nD) Different tasks: expect `read_task_b_modified` and `read_task_b_exists` to execute");
  // Task `read_task`, `read_task_b_modified` and `read_task_b_exists` are different, due to their `Eq` implementation
  // determining that their paths and stampers are different. Therefore, `read_task_b_modified` and `read_task_b_exists`
  // are new tasks, and must be executed.
  let output = context.require_task(&read_task_b_modified)?;
  assert_eq!(&output, "Test");
  let output = context.require_task(&read_task_b_exists)?;
  assert_eq!(&output, "Test");
