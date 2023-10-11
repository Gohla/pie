

#[cfg(test)]
mod test {
  use std::fs::write;
  use std::io::{self, Read};

  use dev_shared::{create_temp_file, write_until_modified};

  use crate::context::non_incremental::NonIncrementalContext;

  use super::*;

  /// Task that reads file at given path and returns it contents as a string.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct ReadStringFromFile(PathBuf);

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
  fn test_file_dependency_consistency() -> Result<(), io::Error> {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file()?;
    write(&temp_file, "test1")?;

    let file_dependency = FileDependency::new(temp_file.path(), FileStamper::Modified)?;
    let dependency: Dependency<ReadStringFromFile, String> = Dependency::RequireFile(file_dependency.clone());
    assert!(file_dependency.is_inconsistent()?.is_none());
    assert!(dependency.is_inconsistent(&mut context)?.is_none());

    // Change the file, changing the stamp the stamper will create next time, making the file dependency inconsistent.
    write_until_modified(&temp_file, "test2")?;
    assert!(file_dependency.is_inconsistent()?.is_some());
    assert!(dependency.is_inconsistent(&mut context)?.is_some());

    Ok(())
  }

  #[test]
  fn test_task_dependency_consistency() -> Result<(), io::Error> {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file()?;
    write(&temp_file, "test1")?;
    let task = ReadStringFromFile(temp_file.path().to_path_buf());
    let output = context.require_task(&task);

    let task_dependency = TaskDependency::new(task.clone(), OutputStamper::Equals, output);
    let dependency = Dependency::RequireTask(task_dependency.clone());
    assert!(task_dependency.is_inconsistent(&mut context).is_none());
    assert!(dependency.is_inconsistent(&mut context)?.is_none());

    // Change the file, causing the task to return a different output, changing the stamp the stamper will create next
    // time, making the task dependency inconsistent.
    write_until_modified(&temp_file, "test2")?;
    assert!(task_dependency.is_inconsistent(&mut context).is_some());
    assert!(dependency.is_inconsistent(&mut context)?.is_some());

    Ok(())
  }
}
