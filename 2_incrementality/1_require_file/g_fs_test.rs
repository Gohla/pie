#[cfg(test)]
mod test {
  use std::fs::remove_file;
  use std::io;

  use assert_matches::assert_matches;

  use dev_shared::{create_temp_dir, create_temp_file};

  use super::*;

  #[test]
  fn test_metadata_ok() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    let metadata = metadata(temp_file)?;
    assert_matches!(metadata, Some(metadata) => {
      assert!(metadata.is_file());
    });
    Ok(())
  }

  #[test]
  fn test_metadata_none() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    remove_file(&temp_file)?;
    let metadata = metadata(&temp_file)?;
    assert!(metadata.is_none());
    Ok(())
  }

  #[test]
  fn test_open_if_file() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    let file = open_if_file(&temp_file)?;
    assert!(file.is_some());
    Ok(())
  }

  #[test]
  fn test_open_if_file_non_existent() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    remove_file(&temp_file)?;
    let file = open_if_file(&temp_file)?;
    assert!(file.is_none());
    Ok(())
  }

  #[test]
  fn test_open_if_file_on_directory() -> Result<(), io::Error> {
    let temp_dir = create_temp_dir()?;
    let file = open_if_file(temp_dir)?;
    assert!(file.is_none());
    Ok(())
  }
}
