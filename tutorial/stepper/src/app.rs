use std::path::{Path, PathBuf};

use crate::modification::{add, apply_diff, create, create_diff, create_diff_builder, insert};
use crate::output::{CargoOutput, DirectoryStructure, SourceArchive};
use crate::stepper::Stepper;

pub fn step_all(
  destination_root_directory: impl AsRef<Path>,
  use_local_pie_graph: bool,
  run_cargo: bool,
  create_outputs: bool,
) {
  let destination_root_directory = destination_root_directory.as_ref();
  let mut stepper = Stepper::new(
    "../src/",
    destination_root_directory,
    destination_root_directory.join("pie").join("src"),
    "../src/gen/",
    run_cargo,
    ["build"],
    create_outputs,
  );

  let pie_graph_path = PathBuf::from("../../graph");
  // Use dunce to not make an absolute path prefixed with "\\?\" (UNC path) on Windows, as Cargo does not support these.
  let pie_graph_path = dunce::canonicalize(pie_graph_path)
    .expect("failed to get absolute path to pie_graph");
  let pie_graph_dependency = if use_local_pie_graph {
    format!("pie_graph = {{ path = '{}' }}", pie_graph_path.display())
  } else {
    r#"pie_graph = "0.0.1""#.to_string()
  };
  stepper.add_substitution("%%%PIE_GRAPH_DEPENDENCY%%%", pie_graph_dependency);

  stepper.with_path("1_programmability", |stepper| {
    stepper.with_path("0_setup", |stepper| {
      stepper
        .apply([
          add("Cargo.toml", "../Cargo.toml"),
          create("lib.rs"),
        ])
        .output([
          CargoOutput::new("cargo.txt"),
          SourceArchive::new("source.zip"),
        ]);
    });
    stepper.with_path("1_api", |stepper| {
      stepper
        .apply([
          add("a_api.rs", "lib.rs"),
        ])
        .output([
          CargoOutput::new("a_cargo.txt"),
          SourceArchive::new("source.zip"),
        ]);
    });
    stepper.with_path("2_non_incremental", |stepper| {
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
      stepper.apply(apply_diff("g_test_correct.rs.diff", "context/non_incremental.rs"))
        .output(SourceArchive::new("source.zip"));
    });
  });

  stepper.with_path("2_incrementality", |stepper| {
    stepper.with_path("1_require_file", |stepper| {
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
          .original("../../1_programmability/2_non_incremental/c_non_incremental_context.rs") // HACK: Explicitly set original file to the one without tests
          .into_modification(),
      ])
        .output([
          DirectoryStructure::new("../../", "e_dir.txt"),
          SourceArchive::new("source.zip"),
        ]);
    });
    stepper.with_path("2_stamp", |stepper| {
      stepper.apply([
        create_diff("a_module.rs", "lib.rs"),
        add("b_file.rs", "stamp.rs"),
        add("c_output.rs", "stamp.rs"),
      ]);
      stepper.apply_may_fail([
        add("d1_test.rs", "stamp.rs"),
      ]);
      stepper.apply([
        create_diff_builder("d2_test_utilities.rs", "../../dev_shared/src/lib.rs")
          .context_length(10)
          .into_modification(),
        create_diff_builder("d3_test_correct.rs", "stamp.rs")
          .context_length(20)
          .into_modification(),
      ]);
      stepper.apply([
        create_diff_builder("e_context_file.rs", "lib.rs")
          .context_length(20)
          .into_modification(),
        create_diff_builder("f_context_task.rs", "lib.rs")
          .into_modification(),
        create_diff("g_non_incremental_context.rs", "context/non_incremental.rs"),
      ]).output(SourceArchive::new("source.zip"));
    });
    stepper.with_path("3_dependency", |stepper| {
      let dest = "dependency.rs";
      stepper.apply([
        create_diff("a_module.rs", "lib.rs"),
        add("b_file.rs", dest),
        add("c_task.rs", dest),
        add("d_dependency.rs", dest),
        add("e_test.rs", dest),
      ]).output(SourceArchive::new("source.zip"));
    });
    stepper.with_path("4_store", |stepper| {
      let dest = "store.rs";
      stepper.apply([
        create_diff("a_Cargo.toml", "../Cargo.toml"),
        create_diff("b_module.rs", "lib.rs"),
        add("c_basic.rs", dest),
        create_diff_builder("d1_mapping_diff.rs", dest)
          .context_length(20)
          .into_modification(),
        create_diff_builder("d2_mapping_diff.rs", dest)
          .context_length(20)
          .into_modification(),
        add("e_mapping.rs", dest),
        add("f_output.rs", dest),
        add("g_dependency.rs", dest),
        add("h_reset.rs", dest),
        add("i_test_file_mapping.rs", dest),
        insert("j_test_task_mapping.rs", "}", dest),
        insert("k_test_task_output.rs", "}", dest),
        insert("l_test_dependencies.rs", "}", dest),
        insert("m_test_reset.rs", "}", dest),
      ]).output(SourceArchive::new("source.zip"));
    });
    stepper.with_path("5_context", |stepper| {
      let dest = "context/top_down.rs";
      stepper.apply([
        create_diff("a_module.rs", "context/mod.rs"),
        add("b_basic.rs", dest),
        create_diff_builder("c_current.rs", dest)
          .context_length(8)
          .into_modification(),
        create_diff("d_file.rs", dest),
        create_diff("e_task.rs", dest),
        create_diff("f_task_dep.rs", dest),
        create_diff("g_check.rs", dest),
        create_diff("h_error_field.rs", dest),
        create_diff("i_error_store.rs", dest),
      ]);
    });
    stepper.with_path("5b_context_example", |stepper| {
      let dest = "../examples/incremental.rs";
      stepper.set_cargo_args(["run", "--example", "incremental"]);
      stepper.apply([
        add("a_task.rs", dest),
        add("b_read_task.rs", dest),
        add("c_write_task.rs", dest),
        add("d_main.rs", dest),
      ]).output(CargoOutput::new("d_main.txt"));
      let insertion_place = "Ok(())";
      stepper.apply([
        insert("e_reuse.rs", insertion_place, dest),
      ]).output(CargoOutput::new("e_reuse.txt"));
      stepper.apply([
        insert("f_file_dep.rs", insertion_place, dest),
        insert("g_new_task.rs", insertion_place, dest),
        insert("h_file_and_task_dep.rs", insertion_place, dest),
        insert("i_early_cutoff.rs", insertion_place, dest),
        insert("j_regen_file.rs", insertion_place, dest),
        insert("k_diff_task.rs", insertion_place, dest),
        insert("l_diff_stamp.rs", insertion_place, dest),
      ]).output([
        CargoOutput::new("l_diff_stamp.txt"),
        SourceArchive::new("source.zip"),
      ]);
      stepper.set_cargo_args(["test"]);
    });
  });
}
