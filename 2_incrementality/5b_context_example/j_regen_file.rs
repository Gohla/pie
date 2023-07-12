  write_until_modified(&output_file, "")?;
  println!("\nG) Regenerate changed output file: expect only `write_task` to execute");
  // The file dependency of `write_task` to `output_file` is inconsistent.
  context.require_task(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

  write_until_modified(&output_file, "")?;
  remove_file(&output_file)?;
  println!("\nH) Regenerate deleted output file: expect only `write_task` to execute");
  // Same results when `output_file` is deleted.
  context.require_task(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");
