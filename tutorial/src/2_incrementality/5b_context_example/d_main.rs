fn main() -> Result<(), io::Error> {
  let temp_dir = create_temp_dir()?;
  let input_file = temp_dir.path().join("input.txt");
  write(&input_file, "")?;
  let output_file = temp_dir.path().join("output.txt");

  let mut context = TopDownContext::new();
  let read_task = ReadStringFromFile::new(&input_file, FileStamper::Modified);
  let write_task = WriteStringToFile::new(read_task.clone(), &output_file, FileStamper::Modified);

  println!("A) New task: expect `read_task` to execute");
  // `read_task` is new, meaning that we have no cached output for it, thus it must be executed.
  let output = context.require_task(&read_task)?;
  assert_eq!(&output, "");
  
  Ok(())
}
