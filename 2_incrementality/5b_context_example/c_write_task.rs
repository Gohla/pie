/// Pseudo-task that writes a string to a file, where the string is provided by another task. The string provider is 
/// boxed to prevent a cyclic definition of infinite size, due to this type being used in [`FileTask`].
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct WriteStringToFile(Box<FileTask>, PathBuf, FileStamper);

impl WriteStringToFile {
  fn new(string_provider: impl Into<Box<FileTask>>, path: impl Into<PathBuf>, stamper: FileStamper) -> FileTask {
    FileTask::WriteStringToFile(Self(string_provider.into(), path.into(), stamper))
  }
  fn execute<C: Context<FileTask>>(&self, context: &mut C) -> Result<(), io::ErrorKind> {
    println!("Writing to {} with {:?} stamper", self.1.file_name().unwrap().to_string_lossy(), self.2);
    let string = context.require_task(&self.0)?;
    let mut file = File::create(&self.1).map_err(|e| e.kind())?;
    file.write_all(string.as_bytes()).map_err(|e| e.kind())?;
    context.require_file_with_stamper(&self.1, self.2).map_err(|e| e.kind())?;
    Ok(())
  }
}

