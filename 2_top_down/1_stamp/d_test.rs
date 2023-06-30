#[cfg(test)]
mod test {
  use std::fs;

  use dev_shared::create_temp_file;

  use super::*;

  #[test]
  fn test_exists_file_stamper() {
    let stamper = FileStamper::Exists;
    let temp_file = create_temp_file();
    let stamp = stamper.stamp(&temp_file).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));

    fs::remove_file(&temp_file).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));
  }

  #[test]
  fn test_modified_file_stamper() {
    let stamper = FileStamper::Modified;
    let temp_file = create_temp_file();
    let stamp = stamper.stamp(&temp_file).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));

    fs::write(&temp_file, format!("{:?}", stamp)).expect("failed to write to temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"), "modified stamp is equal after modifying file");

    fs::remove_file(&temp_file).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"), "modified stamp is equal after removing file");
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
}
