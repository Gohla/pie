
  write_until_modified(&input_file_b, "Test Test")?;
  println!("\nE) Different stampers: expect only `read_task_b_modified` to execute");
  // Both `read_task_b_modified` and `read_task_b_exists` read from the same file, but they use different stampers.
  // Therefore, `read_task_b_modified` must be executed because the modified time has changed, but `read_task_b_exists`
  // will not be executed because its file dependency stamper only checks for existence of the file, and the existence
  // of the file has not changed.
  //
  // Note that using an `Exists` stamper for this task does not make a lot of sense, since it will only read the file
  // on first execute and when it is recreated. But this is just to demonstrate different stampers.
  let output = context.require_task(&read_task_b_modified)?;
  assert_eq!(&output, "Test Test");
  let output = context.require_task(&read_task_b_exists)?;
  assert_eq!(&output, "Test");

