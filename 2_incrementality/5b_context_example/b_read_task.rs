/// Pseudo-task that reads a string from a file.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct ReadStringFromFile(PathBuf, FileStamper);

impl ReadStringFromFile {
  fn new(path: impl AsRef<Path>, stamper: FileStamper) -> FileTask {
    FileTask::ReadStringFromFile(Self(path.as_ref().to_path_buf(), stamper))
  }
  fn execute<C: Context<FileTask>>(&self, context: &mut C) -> Result<String, io::ErrorKind> {
    println!("Reading from {} with {:?} stamper", self.0.file_name().unwrap().to_string_lossy(), self.1);
    let file = context.require_file_with_stamper(&self.0, self.1).map_err(|e| e.kind())?;
    if let Some(mut file) = file {
      let mut string = String::new();
      file.read_to_string(&mut string).map_err(|e| e.kind())?;
      Ok(string)
    } else {
      Err(io::ErrorKind::NotFound)
    }
  }
}
