
  write_until_modified(&input_file, "Hello")?;
  println!("\nC) Inconsistent file dependency: expect `read_task` to execute");
  // The file dependency of `read_task` is inconsistent due to the changed modified time of `input_file`.
  let output = context.require_task(&read_task)?;
  assert_eq!(&output, "Hello");
