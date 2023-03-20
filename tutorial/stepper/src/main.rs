use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
  let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
  let stepper = Stepper::new("../src/", temp_dir.path());
  stepper.step_additions([
    Addition::new("api/Cargo.toml", "Cargo.toml"),
    Addition::new("api/lib.rs", "src/lib.rs")
  ]);
  stepper.step_addition(Addition::new("api/non_incremental.rs", "src/lib.rs"))
}

struct Stepper {
  src_root_dir: PathBuf,
  dst_root_dir: PathBuf,
}

impl Stepper {
  pub fn new(src_root_dir: impl Into<PathBuf>, dst_door_dir: impl Into<PathBuf>) -> Self {
    let src_root_dir = src_root_dir.into();
    let dst_root_dir = dst_door_dir.into();
    Self { src_root_dir, dst_root_dir }
  }
}

struct Addition {
  src: PathBuf,
  dst: PathBuf,
}

impl Addition {
  pub fn new(src: impl Into<PathBuf>, dst: impl Into<PathBuf>) -> Self {
    let src = src.into();
    let dst = dst.into();
    Self { src, dst }
  }
}

impl Stepper {
  pub fn step_addition(&self, addition: Addition) {
    self.step_additions([addition]);
  }

  pub fn step_additions(&self, additions: impl IntoIterator<Item=Addition>) {
    for addition in additions {
      let src = self.src_root_dir.join(&addition.src);
      let text = std::fs::read_to_string(&src).expect("failed to read source file to string");
      let dst = self.dst_root_dir.join(&addition.dst);
      std::fs::create_dir_all(dst.parent().unwrap()).expect("failed to create parent directories");
      let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(dst)
        .expect("failed to open file for appending");
      let line_comment_start = line_comment_start(&src);
      write!(file, "{} Additions from {}\n\n", line_comment_start, src.display()).expect("failed to append header to file");
      write!(file, "{}\n\n", text).expect("failed to append to file");
    }
    self.cargo_test();
  }
}

impl Stepper {
  fn cargo_test(&self) {
    Command::new("cargo")
      .args(["+stable", "test"])
      .current_dir(&self.dst_root_dir)
      .spawn()
      .expect("failed to start cargo test")
      .wait()
      .expect("cargo test failed");
  }
}

fn line_comment_start(file: impl AsRef<Path>) -> &'static str {
  if let Some(ext) = file.as_ref().extension() {
    if let Some(ext) = ext.to_str() {
      match ext {
        "rs" => return "//",
        "toml" => return " # ",
        _ => {}
      }
    }
  }
  return "//";
} 
