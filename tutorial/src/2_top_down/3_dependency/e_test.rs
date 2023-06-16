#[cfg(test)]
mod test {
  use std::fs;
  use std::io::Read;

  use tempfile::{NamedTempFile, TempPath};

  use crate::context::non_incremental::NonIncrementalContext;

  use super::*;

  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct ReadStringFromFile(PathBuf);

  impl Task for ReadStringFromFile {
    type Output = String;
    fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
      let mut string = String::new();
      let file = context.require_file(&self.0)
        .expect("failed to require file");
      if let Some(mut file) = file {
        file.read_to_string(&mut string)
          .expect("failed to read from file");
      };
      string
    }
  }

  #[test]
  fn test_file_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let path = create_temp_path();
    fs::write(&path, "test1")
      .expect("failed to write to file");

    let file_dependency = FileDependency::new(&path, FileStamper::Modified)
      .expect("failed to create file dependency");
    let dependency: Dependency<ReadStringFromFile, String> = Dependency::RequireFile(file_dependency.clone());
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&path, "test2")
      .expect("failed to write to file");
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }

  #[test]
  fn test_task_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let path = create_temp_path();
    fs::write(&path, "test1")
      .expect("failed to write to file");
    let task = ReadStringFromFile(path.to_path_buf());
    let output = context.require_task(&task);

    let task_dependency = TaskDependency::new(task.clone(), OutputStamper::Equals, output);
    let dependency = Dependency::RequireTask(task_dependency.clone());
    assert!(task_dependency.is_inconsistent(&mut context).is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&path, "test2")
      .expect("failed to write to file");
    assert!(task_dependency.is_inconsistent(&mut context).is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }

  fn create_temp_path() -> TempPath {
    NamedTempFile::new().expect("failed to create temporary file").into_temp_path()
  }
}
