  write_until_modified(&input_file, "Hello, World!")?;
  println!("\nE) Inconsistent file and task dependency: expect both tasks to execute");
  // The file dependency of `read_task` is inconsistent. Then, the task dependency from `write_task` to `read_task` is 
  // inconsistent because `read_task` now returns `"Hello, World!"` as output instead of "Hello", and thus its equals 
  // output stamp is different.
  context.require_task(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

