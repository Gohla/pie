  println!("\nD) New task, reuse other: expect only `write_task` to execute");
  // write_task` is new, but `read_task` is not new and its file dependency is still consistent.
  context.require_task(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello");

