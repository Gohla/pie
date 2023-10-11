  println!("\nB) Reuse: expect no execution");
  // `read_task` is not new and its file dependency is still consistent. It is consistent because the modified time of
  // `input_file` has not changed, thus the modified stamp is equal.
  let output = context.require_task(&read_task)?;
  assert_eq!(&output, "Hi");
