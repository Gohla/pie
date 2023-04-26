#[cfg(test)]
mod test {
  use std::fs;

  use tempfile::{NamedTempFile, TempPath};

  use super::*;

  #[test]
  fn test_exists_file_stamper() {
    let stamper = FileStamper::Exists;
    let path = create_temp_path();
    let stamp = stamper.stamp(&path).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&path).expect("failed to stamp"));

    fs::remove_file(&path).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&path).expect("failed to stamp"));
  }

  #[test]
  fn test_modified_file_stamper() {
    let stamper = FileStamper::Modified;
    let path = create_temp_path();
    let stamp = stamper.stamp(&path).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&path).expect("failed to stamp"));

    fs::write(&path, "test").expect("failed to write to temporary file");
    assert_ne!(stamp, stamper.stamp(&path).expect("failed to stamp"), "stamp is equal after modifying file");

    fs::remove_file(&path).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&path).expect("failed to stamp"), "stamp is equal after removing file");
  }

  #[test]
  fn test_inconsequential_output_stamper() {
    let stamper = OutputStamper::Inconsequential;
    let stamp = stamper.stamp(&1);
    assert_eq!(stamp, stamper.stamp(&1));
    assert_eq!(stamp, stamper.stamp(&2));
  }

  #[test]
  fn test_equals_output_stamper() {
    let stamper = OutputStamper::Equals;
    let stamp = stamper.stamp(&1);
    assert_eq!(stamp, stamper.stamp(&1));
    assert_ne!(stamp, stamper.stamp(&2));
  }

  fn create_temp_path() -> TempPath {
    NamedTempFile::new().expect("failed to create temporary file").into_temp_path()
  }
}
