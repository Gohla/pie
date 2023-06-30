use std::path::{Path, PathBuf};

use crate::modification::{add, apply_diff, create, create_diff, create_diff_builder, insert};
use crate::output::{CargoOutput, DirectoryStructure};
use crate::stepper::Stepper;

pub fn step_all(
  destination_root_directory: impl AsRef<Path>,
) {
  let mut stepper = Stepper::new(
    "../src/",
    destination_root_directory.as_ref().join("pie").join("src"),
    "../gen/",
    ["build"],
  );
  
  let pie_graph_path = PathBuf::from("../../graph").canonicalize()
    .expect("failed to get absolute path to pie_graph");
  stepper.add_substitution("%%%PIE_GRAPH_DEPENDENCY%%%", r#"pie_graph = "0.1""#, format!("pie_graph = {{ path = \"{}\" }}", pie_graph_path.display()));

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
        create_diff_builder("a_context.rs", "lib.rs")
          .context_length(10)
          .into_modification(),
        create_diff("b_fs_module.rs", "lib.rs"),
        add("c_fs.rs", "fs.rs"),
        add("d_dev_shared_Cargo.toml", "../../dev_shared/Cargo.toml"),
        add("e_dev_shared_lib.rs", "../../dev_shared/src/lib.rs"),
        create_diff("f_Cargo.toml", "../Cargo.toml"),
        add("g_fs_test.rs", "fs.rs"),
        create_diff_builder("h_non_incremental_context.rs", "context/non_incremental.rs")
          .original("../../1_api/1_non_incremental/c_non_incremental_context.rs") // HACK: Explicitly set original file to the one without tests
          .into_modification(),
      ])
        .output(DirectoryStructure::new("../../", "e_dir.txt"));
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
        create_diff_builder("a1_context.rs", "lib.rs")
          .context_length(20)
          .into_modification(),
        create_diff_builder("a2_context.rs", "lib.rs")
          .into_modification(),
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
    stepper.with_path("4_store", |stepper| {
      stepper.apply([
        create_diff("a_Cargo.toml", "../Cargo.toml"),
        create_diff("b_module.rs", "lib.rs"),
        add("c_basic.rs", "store.rs"),
        create_diff_builder("d1_mapping_diff.rs", "store.rs")
          .context_length(20)
          .into_modification(),
        create_diff_builder("d2_mapping_diff.rs", "store.rs")
          .context_length(20)
          .into_modification(),
        add("e_mapping.rs", "store.rs"),
        add("f_output.rs", "store.rs"),
        add("g_dependency.rs", "store.rs"),
        add("h_reset.rs", "store.rs"),
        add("i_test_file_mapping.rs", "store.rs"),
        insert("j_test_task_mapping.rs", "}", "store.rs"),
        insert("k_test_task_output.rs", "}", "store.rs"),
        insert("l_test_dependencies.rs", "}", "store.rs"),
        insert("m_test_reset.rs", "}", "store.rs"),
      ]);
    });
    stepper.with_path("5_context", |stepper| {
      stepper.apply([
        create_diff("a_module.rs", "context/mod.rs"),
        add("b_basic.rs", "context/top_down.rs"),
        create_diff_builder("c_current.rs", "context/top_down.rs")
          .context_length(8)
          .into_modification(),
        create_diff("d_file.rs", "context/top_down.rs"),
        create_diff("e_task.rs", "context/top_down.rs"),
        create_diff("f_task_dep.rs", "context/top_down.rs"),
        create_diff("g_check.rs", "context/top_down.rs"),
        create_diff("h_error_field.rs", "context/top_down.rs"),
        create_diff("i_error_store.rs", "context/top_down.rs"),
      ]);
    });
  });
}
