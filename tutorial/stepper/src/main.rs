use modification::{AddToFile, ApplyDiff, CreateDiffAndApply, CreateFile};
use output::{CargoOutput, DirectoryStructure};
use stepper::Stepper;

mod modification;
mod output;
mod stepper;
mod util;

fn main() {
  let temp_directory = tempfile::tempdir()
    .expect("failed to create temporary directory");
  let mut stepper = Stepper::new(
    "../src/",
    temp_directory.path().join("pie").join("src"),
    "../gen/",
    ["build"],
  );

  stepper.push_chapter("api");
  stepper
    .apply([
      AddToFile::new("0_Cargo.toml", "../Cargo.toml"),
      AddToFile::new("0_api.rs", "lib.rs"),
    ])
    .output(CargoOutput::new("0_cargo.txt"));
  stepper
    .apply([
      CreateDiffAndApply::new("0_api.rs", "1_context_module.rs", "lib.rs", "1_context_module.rs.diff"),
      AddToFile::new("2_non_incremental_module.rs", "context/mod.rs"),
      CreateFile::new("context/non_incremental.rs")
    ])
    .output(DirectoryStructure::new("../", "2_dir.txt"));
  stepper.apply(AddToFile::new("3_non_incremental_context.rs", "context/non_incremental.rs"));
  stepper.set_cargo_args(["test"]);
  stepper
    .apply(AddToFile::new("4_test_1.rs", "context/non_incremental.rs"))
    .output(CargoOutput::new("4_cargo.txt"));
  stepper
    .apply_failure(CreateDiffAndApply::new("4_test_1.rs", "5_test_2.rs", "context/non_incremental.rs", "5_test_2.rs.diff"))
    .output(CargoOutput::new("5_cargo.txt"));
  stepper
    .apply_failure(CreateDiffAndApply::new("5_test_2.rs", "6_test_2.rs", "context/non_incremental.rs", "6_test_2.rs.diff"))
    .output(CargoOutput::new("6_cargo.txt"));
  stepper.apply(ApplyDiff::new("7_test_2.rs.diff", "context/non_incremental.rs"));
  stepper.pop_chapter();
}
