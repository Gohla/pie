use crate::modification::{add, apply_diff, create, create_diff, create_diff_builder};
use crate::output::{CargoOutput, DirectoryStructure};
use crate::stepper::Stepper;

pub fn run() {
  let temp_directory = tempfile::tempdir()
    .expect("failed to create temporary directory");
  let mut stepper = Stepper::new(
    "../src/",
    temp_directory.path().join("pie").join("src"),
    "../gen/",
    ["build"],
  );

  stepper.push_path("api");
  stepper
    .apply([
      add("0_Cargo.toml", "../Cargo.toml"),
      add("0_api.rs", "lib.rs"),
    ])
    .output(CargoOutput::new("0_cargo.txt"));
  stepper
    .apply([
      create_diff("1_context_module.rs", "lib.rs"),
      add("2_non_incremental_module.rs", "context/mod.rs"),
      create("context/non_incremental.rs"),
    ])
    .output(DirectoryStructure::new("../", "2_dir.txt"));
  stepper.apply(add("3_non_incremental_context.rs", "context/non_incremental.rs"));
  stepper.set_cargo_args(["test"]);
  stepper
    .apply(add("4_test_1.rs", "context/non_incremental.rs"))
    .output(CargoOutput::new("4_cargo.txt"));
  stepper
    .apply_failure(create_diff("5_test_2.rs", "context/non_incremental.rs"))
    .output(CargoOutput::new("5_cargo.txt"));
  stepper
    .apply_failure(create_diff("6_test_2.rs", "context/non_incremental.rs"))
    .output(CargoOutput::new("6_cargo.txt"));
  stepper.apply(apply_diff("7_test_2.rs.diff", "context/non_incremental.rs"));
  stepper.pop_path();

  stepper.with_path("top_down", |stepper| {
    stepper.with_path("0_require_file", |stepper| {
      stepper.apply([
        create_diff("a_context.rs", "lib.rs"),
        create_diff("b_fs_module.rs", "lib.rs"),
        add("c_fs.rs", "fs.rs"),
        create_diff("d_Cargo.toml", "../Cargo.toml"),
        add("e_fs_test.rs", "fs.rs"),
        create_diff_builder("f_non_incremental_context.rs", "context/non_incremental.rs")
          .original("../../api/3_non_incremental_context.rs") // HACK: Explicitly set original file to the one without tests
          .into_modification(),
      ]);
    });
    stepper.with_path("1_stamp", |stepper| {
      stepper.apply([
        create_diff("a_module.rs", "lib.rs"),
        add("b_file.rs", "stamp.rs"),
        add("c_output.rs", "stamp.rs"),
        add("d_test.rs", "stamp.rs"),
      ]);
    });
    stepper.with_path("2_stamp_context", |stepper| {
      stepper.apply([
        create_diff("a_context.rs", "lib.rs"),
        create_diff("b_non_incremental_context.rs", "context/non_incremental.rs"),
      ]);
    });
    stepper.with_path("3_dependency", |stepper| {
      stepper.apply([
        create_diff("a_module.rs", "lib.rs"),
        add("b_file.rs", "dependency.rs"),
        add("c_task.rs", "dependency.rs"),
        add("d_dependency.rs", "dependency.rs"),
        add("e_test.rs", "dependency.rs"),
      ]);
    });
  });
}
