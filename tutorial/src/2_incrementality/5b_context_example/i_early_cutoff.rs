  write_until_modified(&input_file, "Hello, World!")?; // Note: writing same file contents!
  println!("\nF) Early cutoff: expect only `read_task` to execute");
  // File dependency of `read_task` is inconsistent because the modified time changed, but it returns the same output 
  // `"Hello, World!"` because the contents of the file have not actually changed. Then, the task dependency from 
  // `write_task` to `read_task` is consistent because its output did not change, and thus the equality output stamp is 
  // the same.
  context.require_task(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

