

#[cfg(test)]
mod test {
  use std::fs::remove_file;
  use std::io;

  use dev_shared::{create_temp_file, write_until_modified};

  use super::*;

  #[test]
  fn test_exists_file_stamper() -> Result<(), io::Error> {
    let stamper = FileStamper::Exists;
    let temp_file = create_temp_file()?;
    let stamp = stamper.stamp(&temp_file)?;
    assert_eq!(stamp, stamper.stamp(&temp_file)?);

    remove_file(&temp_file)?;
    assert_ne!(stamp, stamper.stamp(&temp_file)?);

    Ok(())
  }

  #[test]
  fn test_modified_file_stamper() -> Result<(), io::Error> {
    let stamper = FileStamper::Modified;
    let temp_file = create_temp_file()?;
    let stamp = stamper.stamp(&temp_file)?;
    assert_eq!(stamp, stamper.stamp(&temp_file)?);

    // Write until file modified time changes. Required on some OSs due to imprecise modified timer causing the modified
    // stamp to be the same after fast consecutive writes.
    write_until_modified(&temp_file, format!("{:?}", stamp))?;
    let new_stamp = stamper.stamp(&temp_file)?;
    assert_ne!(stamp, new_stamp);
    let stamp = new_stamp;

    remove_file(&temp_file)?;
    assert_ne!(stamp, stamper.stamp(&temp_file)?);

    Ok(())
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
