#[cfg(test)]
mod test {
  use std::fs;
  use std::io::Read;
  use std::path::Path;

  use dev_shared::create_temp_file;

  use crate::context::non_incremental::NonIncrementalContext;

  use super::*;

  /// Task that reads file at given path and returns it contents as a string.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  pub struct ReadStringFromFile(PathBuf);

  impl ReadStringFromFile {
    pub fn new(path: impl AsRef<Path>) -> Self { Self(path.as_ref().to_path_buf()) }
  }

  impl Task for ReadStringFromFile {
    type Output = String;
    fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
      let mut string = String::new();
      let file = context.require_file(&self.0).expect("failed to require file");
      if let Some(mut file) = file {
        file.read_to_string(&mut string).expect("failed to read from file");
      };
      string
    }
  }

  #[test]
  fn test_file_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file();
    fs::write(&temp_file, "test1")
      .expect("failed to write to file");

    let file_dependency = FileDependency::new(temp_file.path(), FileStamper::Modified)
      .expect("failed to create file dependency");
    let dependency: Dependency<ReadStringFromFile, String> = Dependency::RequireFile(file_dependency.clone());
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&temp_file, "test2")
      .expect("failed to write to file");
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }

  #[test]
  fn test_task_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file();
    fs::write(&temp_file, "test1")
      .expect("failed to write to file");
    let task = ReadStringFromFile::new(&temp_file);
    let output = context.require_task(&task);

    let task_dependency = TaskDependency::new(task.clone(), OutputStamper::Equals, output);
    let dependency = Dependency::RequireTask(task_dependency.clone());
    assert!(task_dependency.is_inconsistent(&mut context).is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&temp_file, "test2")
      .expect("failed to write to file");
    assert!(task_dependency.is_inconsistent(&mut context).is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }
}
