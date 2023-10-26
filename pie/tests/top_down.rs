use std::fs::{remove_file, write};
use std::ops::RangeInclusive;

use assert_matches::assert_matches;
use testresult::TestResult;

use dev_ext::task::*;
use dev_util::{create_temp_dir, write_until_modified};
use pie::{Context, Task};
use pie::resource::file::{ExistsChecker, FsError, ModifiedChecker};
use pie::task::AlwaysConsistent;
use pie::tracker::event::*;

use crate::util::{new_test_pie, TestPieExt};

mod util;

#[test]
fn execute() {
  let mut pie = new_test_pie();
  let task = Constant("Hello, World!");
  let output = pie.require_then_assert(&task, |tracker| {
    let events = tracker.slice();
    assert_matches!(events.get(0), Some(Event::BuildStart));
    assert_matches!(events.get(1), Some(Event::RequireStart(e)) if e.task_equals(&task));
    assert_matches!(events.get(2), Some(Event::ExecuteStart(e)) if e.task_equals(&task));
    assert_matches!(events.get(3), Some(Event::ExecuteEnd(e)) if e.task_equals(&task));
    assert_matches!(events.get(4), Some(Event::RequireEnd(e)) if e.task_equals(&task));
    assert_matches!(events.get(5), Some(Event::BuildEnd));
  });
  assert_eq!(output, "Hello, World!");
}

#[test]
fn reuse() {
  let mut pie = new_test_pie();
  let task = Constant("Hello, World!");
  // New task: execute.
  let output = pie.require(&task);
  assert_eq!(output, "Hello, World!");
  // Nothing changed: no execute
  pie.require_then_assert_no_execute(&task);
}

#[test]
fn read_file() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in.txt");
  write(&file, "HELLO WORLD!")?;
  let task = ReadFile::new(&file).with_checker(ModifiedChecker);

  // 1) Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(&output, "HELLO WORLD!");
  // 2) Require task again and assert that it is not executed because its file dependency consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(&output, "HELLO WORLD!");
  // 3) Change required file such that the file dependency of the task becomes inconsistent.
  write_until_modified(&file, "!DLROW OLLEH")?;
  // 4) Require task again and assert that it is re-executed because its file dependency is inconsistent.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(&output, "!DLROW OLLEH");

  // Repeat the test with `ExistsChecker`, which results in a different outcome.
  write(&file, "HELLO WORLD!")?;
  let task = ReadFile::new(&file).with_checker(ExistsChecker);

  // 1) Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(&output, "HELLO WORLD!");
  // 2) Require task again and assert that it is not executed because its file dependency is consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(&output, "HELLO WORLD!");
  // 3) Change required file, but the file dependency of the task stays consistent.
  write_until_modified(&file, "!DLROW OLLEH")?;
  // 4) Require task again and assert that it is not executed because its file dependency is still consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(&output, "HELLO WORLD!");

  Ok(())
}

#[test]
fn require_task() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in.txt");
  write(&file, "HELLO WORLD!")?;
  let read = ReadFile::new(&file);
  let lower = ToLower(read.clone());

  /// Assert that task requires and executes are temporally sound.
  fn assert_task_temporally_sound(require: &RangeInclusive<usize>, execute: &RangeInclusive<usize>) {
    // Require and execute ends come after require and execute starts.
    assert!(require.end() > require.start());
    assert!(execute.end() > execute.start());
    // Task require ends should be later than their executes.
    assert!(require.end() > execute.end());
  }

  // 1) Require `ToLower` and assert that both tasks are executed in dependency order, because both tasks are new:
  // → ToLower
  //   ▶ ToLower [reason: new]
  //     → ReadFile
  //       ▶ ReadFile [reason: new]
  //         - `file`
  //       ◀ Ok(String("HELLO WORLD!"))
  //     ← Ok(String("HELLO WORLD!"))
  //   ◀ Ok(String("hello world!"))
  // ← Ok(String("hello world!"))
  // 🏁
  let output = pie.require_then_assert(&lower, |tracker| {
    // `ToLower` is required and executed, and its require and execute are temporally sound.
    let lower_require = assert_matches!(tracker.first_require_range(&lower), Some(r) => r);
    let lower_execute = assert_matches!(tracker.first_execute_range(&lower), Some(r) => r);
    assert_task_temporally_sound(&lower_require, &lower_execute);

    // `ReadFile` is required and executed, and its require and execute are temporally sound.
    let read_require = assert_matches!(tracker.first_require_range(&read), Some(r) => r);
    let read_execute = assert_matches!(tracker.first_execute_range(&read), Some(r) => r);
    assert_task_temporally_sound(&read_require, &read_execute);

    // Sanity check: `file` is required.
    let file_read = assert_matches!(tracker.first_read_end_index(&file), Some(i) => i);

    // `ReadFile` is required while `ToLower` is being required.
    assert!(read_require.start() > lower_require.start());
    assert!(lower_require.end() > read_require.end());

    // `ReadFile` is executed while `ToLower` is being executed.
    assert!(read_execute.start() > lower_execute.start());
    assert!(lower_execute.end() > read_execute.end());

    // Sanity check: `ReadFile` requires `file` while executing.
    assert!(file_read > read_execute.start());
    assert!(read_execute.end() > file_read);
  })?;
  assert_eq!(output.as_str(), "hello world!");

  // 2) Require `ToLower` again and assert that no tasks are executed because all dependencies are consistent:
  // → ToLower
  //   ? ReadFile
  //     ✓ `file`
  //   ✓ ReadFile
  // ← Ok(String("hello world!"))
  // 🏁
  let output = pie.require_then_assert_no_execute(&lower)?;
  assert_eq!(output.as_str(), "hello world!");

  // Change `file` such that the file dependency of `ReadFile` becomes inconsistent.
  write_until_modified(&file, "!DLROW OLLEH")?;

  // 3) Require `ToLower` and assert that both tasks are re-executed in reverse dependency order:
  // → ToLower
  //   ? ReadFile
  //     ✗ `file` [inconsistent: modified file stamp change]
  //     ▶ ReadFile [reason: `file` is inconsistent due to modified file stamp change]
  //       - `file`
  //     ◀ Ok(String("!DLROW OLLEH")) [note: returns a different output!]
  //   ✗ ReadFile [inconsistent: equals output stamp change]
  //   ▶ ToLower [reason: ReadFile is inconsistent due to equals output stamp change]
  //     → ReadFile
  //     ← Ok(String("!DLROW OLLEH")) [note: skipped checking `read` because it is already consistent this session!]
  //   ◀ Ok(String("!dlrow olleh"))
  // ← Ok(String("!dlrow olleh"))
  // 🏁
  let output = pie.require_then_assert(&lower, |tracker| {
    // Sanity checks: `ToLower` and `ReadFile` are required and executed, and `file` is required.
    let lower_require = assert_matches!(tracker.first_require_range(&lower), Some(r) => r);
    let lower_execute = assert_matches!(tracker.first_execute_range(&lower), Some(r) => r);
    assert_task_temporally_sound(&lower_require, &lower_execute);
    let read_require = assert_matches!(tracker.first_require_range(&read), Some(r) => r);
    let read_execute = assert_matches!(tracker.first_execute_range(&read), Some(r) => r);
    assert_task_temporally_sound(&read_require, &read_execute);
    let file_read = assert_matches!(tracker.first_read_end_index(&file), Some(i) => i);

    // Sanity check: `ReadFile` reads `file` while executing.
    assert!(file_read > read_execute.start());
    assert!(read_execute.end() > file_read);

    // `ToLower` is executed after `ReadFile` has been executed.
    assert!(lower_execute.start() > read_execute.end());
    // `ReadFile` is executed while `ToLower` is being required.
    assert!(read_execute.start() > lower_require.start());
    assert!(lower_require.end() > read_execute.end());
  })?;
  assert_eq!(output.as_str(), "!dlrow olleh");

  // Change `file` such that the file dependency of `ReadFile` becomes inconsistent, but still has the same content.
  write_until_modified(&file, "!DLROW OLLEH")?;

  let output = pie.require_then_assert(&lower, |tracker| {
    // `ReadFile` needs to be executed due to its `file` dependency being inconsistent (modified stamp changed).
    assert!(tracker.one_execute_of(&read));
    // `ToLower` is not executed, because its task dependency to `ReadFile` is consistent (equals stamp is the same).
    assert!(!tracker.any_execute_of(&lower));
  })?;
  assert_eq!(output.as_str(), "!dlrow olleh");

  Ok(())
}

#[test]
fn no_superfluous_task_dependencies() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in.txt");
  write(&file, "Hello, World!")?;
  let read = ReadFile::new(&file);
  let lower = ToLower(read.clone());
  let upper = ToUpper(lower.clone());

  // Require `ToLower` and assert that `ReadFile` and `ToLower` are executed because they are new, but not `ToUpper`,
  // because it not required by anything. `ToLower` will return `"hello, world!"`.
  let output = pie.require_then_assert(&lower, |tracker| {
    assert!(tracker.one_execute_of(&read));
    assert!(tracker.one_execute_of(&lower));
    assert!(!tracker.any_execute_of(&upper));
  })?;
  assert_eq!(output.as_str(), "hello, world!");

  // Require `ToUpper` and assert that it is executed because it is new, but not `ReadFile` nor `ToLower` because their
  // dependencies are consistent.
  let output = pie.require_then_assert(&upper, |tracker| {
    assert!(!tracker.any_execute_of(&read));
    assert!(!tracker.any_execute_of(&lower));
    assert!(tracker.one_execute_of(&upper));
  })?;
  assert_eq!(output.as_str(), "HELLO, WORLD!");

  Ok(())
}


// Cycle detection tests.

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum Cycle {
  RequireSelf,
  RequireA,
  RequireB,
}
impl Task for Cycle {
  type Output = ();
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    match self {
      Self::RequireSelf => context.require(&Self::RequireSelf, AlwaysConsistent),
      Self::RequireA => context.require(&Self::RequireB, AlwaysConsistent),
      Self::RequireB => context.require(&Self::RequireA, AlwaysConsistent),
    }
  }
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_panics() {
  let mut pie = new_test_pie();
  pie.require(&Cycle::RequireSelf);
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_a_panics() {
  let mut pie = new_test_pie();
  pie.require(&Cycle::RequireA);
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_b_panics() {
  let mut pie = new_test_pie();
  pie.require(&Cycle::RequireB);
}


// Hidden dependency detection tests.

#[test]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_read_panics() {
  fn run() -> TestResult {
    let mut pie = new_test_pie();
    let temp_dir = create_temp_dir()?;
    let file = temp_dir.path().join("in_out.txt");

    pie.run_in_session(|mut session| {
      let providing_task = WriteFile::new(Constant::new_ok("Test 1"), &file);
      session.require(&providing_task)?;
      let requiring_task = ReadFile::new(&file);
      session.require(&requiring_task)?;
      Ok::<(), FsError>(())
    })?;

    Ok(())
  }
  run().unwrap();
}

#[test]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_write_panics() {
  fn run() -> TestResult {
    let mut pie = new_test_pie();
    let temp_dir = create_temp_dir()?;
    let file = temp_dir.path().join("in_out.txt");
    write(&file, "test")?;

    pie.run_in_session(|mut session| {
      let requiring_task = ReadFile::new(&file);
      session.require(&requiring_task)?;
      let providing_task = WriteFile::new(Constant::new_ok("Test 1"), &file);
      session.require(&providing_task)?;
      Ok::<(), FsError>(())
    })?;

    Ok(())
  }
  run().unwrap();
}

#[test]
fn non_hidden_dependency() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in_out.txt");
  write(&file, "Hello, World!")?;

  let input_file = temp_dir.path().join("in.txt");
  write(&input_file, "Hi There!")?;
  let read_input = ReadFile::new(&input_file);
  let write = WriteFile::new(read_input.clone(), &file);
  let read = ReadFile::new(&file).with_origin(write.clone());

  // Require `read`, which requires `write` to update the provided file. All tasks are executed because they are new.
  let output = pie.require_then_assert(&read, |tracker| {
    assert!(tracker.one_execute_of(&read));
    assert!(tracker.one_execute_of(&write));
    assert!(tracker.one_execute_of(&read_input));
  })?;
  // `read` should output what `write` wrote, which is what `read_input` read from `input_file`.
  assert_eq!(output.as_str(), "Hi There!");

  // First ensure the modified date of `file` has changed, then remove `file`.
  write_until_modified(&file, "Hi There!")?;
  remove_file(&file)?;
  assert!(!file.exists());

  // Confirm the provided file is re-generated.
  let output = pie.require_then_assert(&read, |tracker| {
    // `write` should execute to re-generate the provided file.
    assert!(tracker.one_execute_of(&write));
    // `read_input` is not executed because its file dependency to `input_file` is consistent.
    assert!(!tracker.any_execute_of(&read_input));
    // `read` is executed because its `file` dependency is inconsistent, due to it having a new modified date. If we use
    // a file hash stamper, we can prevent this re-execution.
    assert!(tracker.one_execute_of(&read));
  })?;
  assert!(file.exists());
  assert_eq!(output.as_str(), "Hi There!");

  // Change `read_input` and confirm the change is propagated to `read`.
  write_until_modified(&input_file, "Hello There!")?;
  let output = pie.require(&read)?;
  assert_eq!(output.as_str(), "Hello There!");

  Ok(())
}


// Overlapping write detection tests.

#[test]
#[should_panic(expected = "Overlapping write")]
fn overlapping_write_panics() {
  fn run() -> TestResult {
    let mut pie = new_test_pie();
    let temp_dir = create_temp_dir()?;
    let file = temp_dir.path().join("out.txt");

    pie.run_in_session(|mut session| {
      let task_1 = WriteFile::new(Constant::new_ok("Test 1"), &file);
      session.require(&task_1)?;
      let task_2 = WriteFile::new(Constant::new_ok("Test 2"), &file);
      session.require(&task_2)?;
      Ok::<(), FsError>(())
    })?;

    Ok(())
  }
  run().unwrap();
}

#[test]
fn same_task_no_overlap() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let output_file = temp_dir.path().join("out.txt");
  let input_file = temp_dir.path().join("in.txt");
  write(&input_file, "Hello, World!")?;

  let read = ReadFile::new(&input_file);
  let write = WriteFile::new(read, &output_file);

  pie.require_then_assert_one_execute(&write)?;
  // Requiring and executing the same task does not cause overlap.
  write_until_modified(&input_file, "World, Hello?")?;
  pie.require_then_assert_one_execute(&write)?;
  // Even when required indirectly.
  write_until_modified(&input_file, "Hello, World!")?;
  pie.require_then_assert_one_execute(&Require::new(write))?;

  Ok(())
}
