use std::path::{Path, PathBuf};

use crate::modification::{add, create, create_diff, create_diff_builder, create_diff_from_destination_file, insert};
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
      stepper.apply(create_diff("g_remove_test.rs", "context/non_incremental.rs"));
      stepper.apply(create_diff("h_test_correct.rs", "context/non_incremental.rs"))
        .output(SourceArchive::new("source.zip"));
    });
  });

  stepper.with_path("2_incrementality", |stepper| {
    stepper.with_path("1_require_file", |stepper| {
      stepper.apply([
        create_diff("a_context.rs", "lib.rs"),
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
        create_diff("d2_test_utilities.rs", "../../dev_shared/src/lib.rs"),
        create_diff("d3_test_correct.rs", "stamp.rs"),
      ]);
      stepper.apply([
        create_diff("e_context_file.rs", "lib.rs"),
        create_diff("f_context_task.rs", "lib.rs"),
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
        create_diff("d1_mapping_diff.rs", dest),
        create_diff("d2_mapping_diff.rs", dest),
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
        create_diff("c_current.rs", dest),
        create_diff("d_file.rs", dest),
        create_diff("e_task.rs", dest),
        create_diff("f_task_dep.rs", dest),
        create_diff("g_check.rs", dest),
        create_diff("h_error_field.rs", dest),
        create_diff("i_error_store.rs", dest),
      ]).output(SourceArchive::new("source.zip"));
    });
    stepper.with_path("6_example", |stepper| {
      let dest = "../examples/incremental.rs";
      stepper.set_cargo_args(["run", "--example", "incremental"]);
      stepper.apply([
        add("a_task.rs", dest),
        add("b_main.rs", dest),
      ]).output(CargoOutput::new("b_main.txt"));
      let insertion_place = "  Ok(())";
      stepper.apply([
        insert("c_reuse.rs", insertion_place, dest),
      ]).output(CargoOutput::new("c_reuse.txt"));
      stepper.apply([
        insert("d_file_dep.rs", insertion_place, dest),
        insert("e_diff_task.rs", insertion_place, dest),
        insert("f_diff_stamp.rs", insertion_place, dest),
      ]).output([
        CargoOutput::new("f_diff_stamp.txt"),
        SourceArchive::new("source.zip"),
      ]);
      stepper.set_cargo_args(["test"]);
    });
  });


  stepper.with_path("3_min_sound", |stepper| {
    stepper.with_path("1_session", |stepper| {
      stepper.set_cargo_args(["check"]);
      stepper.apply([
        create_diff("a_lib_import.rs", "lib.rs"),
        add("b_lib_pie_session.rs", "lib.rs"),
      ]);
      stepper.set_cargo_args(["check", "--lib"]);
      stepper.apply([
        create_diff("c_top_down_new.rs", "context/top_down.rs"),
        create_diff("d_top_down_fix.rs", "context/top_down.rs"),
        create_diff_from_destination_file("e_lib_require.rs", "lib.rs"),
        create_diff_from_destination_file("f_lib_private_module.rs", "lib.rs"),
      ]);
      stepper.set_cargo_args(["run", "--example", "incremental"]);
      stepper.apply([
        create_diff_from_destination_file("g_example.rs", "../examples/incremental.rs"),
      ]).output(SourceArchive::new("source.zip"));
      stepper.set_cargo_args(["test"]);
      stepper.apply([
        create_diff_from_destination_file("h_lib_consistent.rs", "lib.rs"),
        create_diff_from_destination_file("i_context_consistent.rs", "context/top_down.rs"),
      ]);
    });

    stepper.with_path("2_tracker", |stepper| {
      stepper.apply([
        create_diff("a_lib_module.rs", "lib.rs"),
        add("b_tracker.rs", "tracker/mod.rs"),
      ]);
      stepper.apply([
        add("c_noop.rs", "tracker/mod.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("d_lib_tracker.rs", "lib.rs"),
        create_diff_from_destination_file("e_top_down_tracker.rs", "context/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_builder("f_mod_writing.rs", "tracker/mod.rs")
          .original("b_tracker.rs")
          .into_modification(),
        add("g_writing.rs", "tracker/writing.rs"),
        add("h_1_writing_impl.rs", "tracker/writing.rs"),
        add("h_2_writing_impl.rs", "tracker/writing.rs"),
      ]);
      stepper.set_cargo_args(["run", "--example", "incremental"]);
      stepper.apply([
        create_diff("i_writing_example.rs", "../examples/incremental.rs"),
      ]).output(CargoOutput::new("i_writing_example.txt"));
      stepper.set_cargo_args(["test"]);
      stepper.apply([
        create_diff_builder("j_mod_event.rs", "tracker/mod.rs")
          .original("f_mod_writing.rs")
          .into_modification(),
        add("k_event.rs", "tracker/event.rs"),
        add("l_event_tracker.rs", "tracker/event.rs"),
        add("m_1_event_inspection.rs", "tracker/event.rs"),
        add("m_2_event_inspection.rs", "tracker/event.rs"),
      ]);
      stepper.apply([
        add("n_composite.rs", "tracker/mod.rs"),
      ]).output(SourceArchive::new("source.zip"));
    });

    stepper.with_path("3_test", |stepper| {
      stepper.apply([
        add("a_1_common_pie.rs", "../tests/common/mod.rs"),
        add("a_2_common_ext.rs", "../tests/common/mod.rs"),
        add("a_3_common_task.rs", "../tests/common/mod.rs"),
      ]);
      stepper.apply([
        add("b_test_execute.rs", "../tests/top_down.rs")
      ]);
      stepper.apply([
        add("c_test_reuse.rs", "../tests/top_down.rs")
      ]);

      stepper.run_cargo(["test", "--", "--test-threads=1"], Some(true));
      stepper.run_cargo_applied(["test", "--test", "top_down", "test_reuse"], Some(true))
        .output(CargoOutput::new("c_test_reuse_stdout.txt"));

      stepper.apply([
        create_diff_from_destination_file("d_1_read_task.rs", "../tests/common/mod.rs"),
        create_diff_from_destination_file("d_2_test_require_file.rs", "../tests/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("e_1_lower_task.rs", "../tests/common/mod.rs"),
        create_diff_from_destination_file("e_2_test_require_task.rs", "../tests/top_down.rs"),
        create_diff_from_destination_file("e_3_test_require_task.rs", "../tests/top_down.rs"),
        create_diff_from_destination_file("e_4_test_require_task.rs", "../tests/top_down.rs"),
        create_diff_from_destination_file("e_5_test_require_task.rs", "../tests/top_down.rs"),
        create_diff_from_destination_file("e_6_test_require_task.rs", "../tests/top_down.rs"),
      ]).output(
        SourceArchive::new("source.zip")
      );
    });

    stepper.with_path("4_fix_task_dep", |stepper| {
      stepper.apply([
        create_diff_from_destination_file("a_upper_task.rs", "../tests/common/mod.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("b_test_setup.rs", "../tests/top_down.rs"),
      ]);
      stepper.run_cargo(["test", "--test", "top_down", "test_no_superfluous_task_dependencies"], Some(true));
      stepper.apply_failure([
        create_diff_from_destination_file("c_test_manifest.rs", "../tests/top_down.rs"),
      ]);
      stepper.run_cargo_applied(["test", "--test", "top_down", "test_no_superfluous_task_dependencies"], Some(false)).output([
        CargoOutput::with_modify_fn("c_test_manifest_2.txt", |log|log.split('🏁').nth(1).expect("second build to be in the build log").to_string()),
        CargoOutput::with_modify_fn("c_test_manifest_3.txt", |log|log.split('🏁').nth(2).expect("third build to be in the build log").to_string())
      ]);
      stepper.apply_failure([
        create_diff_from_destination_file("d_1_make_consistent.rs", "context/top_down.rs"),
        create_diff_from_destination_file("d_2_task_dependency.rs", "dependency.rs"),
        create_diff_from_destination_file("d_3_impl.rs", "context/top_down.rs"),
        create_diff_from_destination_file("d_4_non_incremental.rs", "context/non_incremental.rs"),
      ]);
      stepper.run_cargo_applied(["test", "--test", "top_down", "test_require_task"], Some(false)).output([
        CargoOutput::with_modify_fn("e_fix_tests_2.txt", |log|log.split('🏁').nth(1).expect("second build to be in the build log").to_string()),
        CargoOutput::with_modify_fn("e_fix_tests_3.txt", |log|log.split('🏁').nth(2).expect("third build to be in the build log").to_string())
      ]);
      stepper.apply([
        create_diff_from_destination_file("e_fix_tests.rs", "../tests/top_down.rs"),
      ]).output(
        SourceArchive::new("source.zip")
      );
    });

    stepper.with_path("5_overlap", |stepper| {
      stepper.apply([
        create_diff_from_destination_file("a_test_tasks.rs", "../tests/common/mod.rs"),
        create_diff_from_destination_file("b_test_issue.rs", "../tests/top_down.rs"),
        add("c_test_separate.rs", "../tests/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("d_dependency.rs", "dependency.rs"),
        create_diff_from_destination_file("e_1_tracker.rs", "tracker/mod.rs"),
        create_diff_from_destination_file("e_2_writing.rs", "tracker/writing.rs"),
        create_diff_from_destination_file("e_3_event.rs", "tracker/event.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("f_store.rs", "store.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("g_context.rs", "lib.rs"),
        create_diff_from_destination_file("h_non_incr.rs", "context/non_incremental.rs"),
        create_diff_from_destination_file("i_top_down.rs", "context/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("j_1_store.rs", "store.rs"),
        create_diff_from_destination_file("j_2_top_down.rs", "context/top_down.rs"),
      ]);
      stepper.apply_failure([
        create_diff_from_destination_file("k_1_use_provide.rs", "../tests/common/mod.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("k_2_fix_test.rs", "../tests/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("k_3_more_tests.rs", "../tests/top_down.rs"),
      ]).output(
        SourceArchive::new("source.zip")
      );
    });

    stepper.with_path("6_hidden_dep", |stepper| {
      stepper.apply([
        add("a_1_test.rs", "../tests/top_down.rs"),
        create_diff("a_2_test.rs", "../tests/top_down.rs"),
      ]);
      stepper.apply([
        create_diff_from_destination_file("b_1_store.rs", "store.rs"),
        create_diff_from_destination_file("b_2_store.rs", "store.rs"),
      ]);
      stepper.apply_failure(
        create_diff_from_destination_file("c_top_down.rs", "context/top_down.rs")
      );
    });
  });
}
