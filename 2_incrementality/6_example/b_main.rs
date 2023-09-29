
fn main() -> Result<(), io::Error> {
  let temp_dir = create_temp_dir()?;
  let input_file = temp_dir.path().join("input.txt");
  write_until_modified(&input_file, "Hi")?;

  let mut context = TopDownContext::new();
  let read_task = ReadStringFromFile::new(&input_file, FileStamper::Modified);

  println!("A) New task: expect `read_task` to execute");
  // `read_task` is new, meaning that we have no cached output for it, thus it must be executed.
  let output = context.require_task(&read_task)?;
  assert_eq!(&output, "Hi");

  Ok(())
}
