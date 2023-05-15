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

  stepper.with_path("0_setup", |stepper| {
    stepper
      .apply([
        add("Cargo.toml", "../Cargo.toml"),
        create("lib.rs"),
      ])
      .output(CargoOutput::new("cargo.txt"));
  });

  stepper.with_path("1_api", |stepper| {
    stepper.with_path("0_api", |stepper| {
      stepper
        .apply([
          add("a_api.rs", "lib.rs"),
        ])
        .output(CargoOutput::new("a_cargo.txt"));
    });

    stepper.with_path("1_non_incremental", |stepper| {
      stepper
        .apply([
          create_diff("a_context_module.rs", "lib.rs"),
          add("b_non_incremental_module.rs", "context/mod.rs"),
          create("context/non_incremental.rs"),
        ])
        .output(DirectoryStructure::new("../", "b_dir.txt"));
      stepper.apply(add("c_non_incremental_context.rs", "context/non_incremental.rs"));
      stepper.set_cargo_args(["test"]);
      stepper
        .apply(add("d_test.rs", "context/non_incremental.rs"))
        .output(CargoOutput::new("d_cargo.txt"));
      stepper
        .apply_failure(create_diff("e_test_problematic.rs", "context/non_incremental.rs"))
        .output(CargoOutput::new("e_cargo.txt"));
      stepper
        .apply_failure(create_diff("f_test_incompatible.rs", "context/non_incremental.rs"))
        .output(CargoOutput::new("f_cargo.txt"));
      stepper.apply(apply_diff("g_test_correct.rs.diff", "context/non_incremental.rs"));
    });
  });

  stepper.with_path("2_top_down", |stepper| {
    stepper.with_path("0_require_file", |stepper| {
      stepper.apply([
        create_diff("a_context.rs", "lib.rs"),
        create_diff("b_fs_module.rs", "lib.rs"),
        add("c_fs.rs", "fs.rs"),
        create_diff("d_Cargo.toml", "../Cargo.toml"),
        add("e_fs_test.rs", "fs.rs"),
        create_diff_builder("f_non_incremental_context.rs", "context/non_incremental.rs")
          .original("../../1_api/1_non_incremental/c_non_incremental_context.rs") // HACK: Explicitly set original file to the one without tests
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
