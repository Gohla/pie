# Prevent Overlapping File Writes

```admonish warning title="Under construction"
This section is under construction.
```

So far we have only considered reading files in the build system.
However, there are many tasks that also write files.
For example, a C compiler reads a C source file (and header files) and writes an object file with the compiled result, which is typically an intermediate file that gets passed to a linker.
Another example is a file copy task that reads a file and copies it to another file.

We can handle file writes in tasks with `context.require_file`.
However, what should happen when two tasks write to the same file?
In a non-incremental setting, the last writer wins by overwriting (or appending to) the file.
Does this behaviour also occur in our incremental build system?

Unfortunately, this is not always the case in our incremental build system, because we can `require` individual tasks.
This is a bit tricky to explain without an example, so we will first add some testing tasks and write a test that showcases the problem.
In this section, we will continue with:
 
1) Add the `WriteFile` and `Sequence` tasks to the testing tasks.
2) Create a `test_overlapping_file_write` test to showcase the issue.
3) Introduce a new kind of dependency: a _provide file dependency_ for writing to (and creating) files.
4) Prevent overlapping file writes by checking for them at runtime, fixing the issue.

## Add `WriteFile` and `Sequence` tasks

Add the `WriteFile` and `Sequence` tasks to `pie/tests/common/mod.rs`: 

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/a_test_tasks.rs.diff}}
```

`WriteFile` requires a string providing task to produce a string, writes that string to given file, then requires the file with given stamper to create a dependency.
It uses `write_until_modified` to ensure that writes change the modification time, which we need for consistent testing.
`Sequence` requires multiple tasks stored as a `Vec<TestTask>`.
Both return `TestOutput::Unit` when successful, but propagate errors.
`TestOutput::Unit` is like `()`, the unit type with a single value.

Because `TestOutput` now has two variants, the `as_str` and `into_string` methods can now fail with a panic (which is fine for testing).

```admonish question title="Why not use the Inconsequential Stamper?" collapsible=true
`Sequence` ignores `Result::Ok` outputs from required tasks, but it propagates `Result::Err` outputs. 
Therefore, we cannot use the `Inconsequential` output stamper, as it would not re-execute `Sequence` when a task it requires goes from returning `Ok` to `Err`, and vice versa.

We could, however, implement a stamper that ignores changes to the `Ok` variant of results, but not the `Err` variant, to increase incrementality.
```

## Test to showcase the issue

Now we write a test to showcase the issue.
Add the following test to `pie/tests/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/b_test_issue.rs.diff}}
```

In this test, we create 2 `WriteFile` tasks that both write to `output_file`.
`write_1` gets the string to write from `ret`, which returns `"Hi there"`.
`write_2` gets the string to write from `read`, which returns the contents of `input_file`.
Both write tasks are put into a `Sequence`, with first `write_1`, then `write_2`.

We require `seq` and assert that `output_file` should contain the result of executing `write_2`, which requires `read` to get the contents of `input_file`.
This result makes sense, it's what would happen in a non-incremental setting.

However, we then modify `input_file` to make `write_2` inconsistent, and then require `write_1` directly instead of requiring `seq`.
The result is that `output_file` now contains `"Hi there"`, even though `write_2` is inconsistent!

This behaviour stems from the fact that we can `require` individual tasks, which is actually a great feature, not a bug!
When we `require` a task, we are asking the build system to make **that task** consistent.
We are **not** asking the build system to make **all** tasks consistent.
The build system recursively checks and executes only the tasks that are absolutely necessary to make that task consistent.
If it would not do that, it would not truly be incremental!
Therefore, we cannot (and shouldn't) get rid of this feature, and instead need to find another solution to this problem.  

As we saw in this test, `output_file` is not in a consistent state, because `write_2` is inconsistent and needs to be executed to bring `output_file` into a consistent state.
However, if `write_2` would write to another file, there would be no inconsistency.
Let's write a test with separate output files.

Add the following test to `pie/tests/top_down.rs`:

```rust,
{{#include c_test_separate.rs}}
```

Here, `write_1` writes to `output_file_1`, and `write_2` writes to `output_file_2`.
Thus, requiring `write_1` makes `output_file_1` consistent.
Requiring `write_2` makes `output_file_2` consistent.
The last two `require_then_assert_no_execute` statements do this, and there are no inconsistencies with these separate output files.
Therefore, to prevent confusion, inconsistencies, and (subtle) incrementality bugs, we will detect overlapping file writes and disallow them.

Before continuing, confirm both tests succeed with `cargo test`.
We will modify the first test to assert the desired behaviour later.

~~~admonish tip title="Reduce Programming Errors by Returning Paths" collapsible=true
In this last test, we can still make a programming error where we read an output file without first requiring the task that makes that output file consistent.
We can mostly solve that by having `WriteFile` return the path it wrote to:

```rust
TestTask::WriteFile(string_provider_task, path, stamper) => {
  let string = context.require_task(string_provider_task.as_ref())?.into_string();
  write_until_modified(path, string.as_bytes()).map_err(|e| e.kind())?;
  context.require_file_with_stamper(path, *stamper).map_err(|e| e.kind())?;
  Ok(TestOutput::Path(path.clone()))
}
```

Then you can have `WriteFile` take ownership of the path so we don't accidentally use it:

```rust
let ret = Return("Hi there");
let write_1 = WriteFile(Box::new(ret.clone()), temp_dir.path().join("out_1.txt"), FileStamper::Modified);
```

And you can read the output file with:

```rust
{
  let output_file = pie.require_then_assert_no_execute(&write_1)?;
  assert_eq!(read_to_string(output_file.as_path())?, "Hi there");
}
```

You can still manually construct the path to the output file and read it to break this, but at least this prevents most accidental reads.
~~~

## Implement provided files

We currently have no means to disallow overlapping file writes.
We only have one kind of file dependency: require file, which is currently used for both reading from and writing to files.
It's perfectly fine to read from a single file from multiple tasks, so we can't disallow multiple tasks from creating a require file dependency to the same file.
Therefore, we must introduce a new kind of dependency for writing to (and creating) files: the _provide file dependency_.
A file may only be provided by one task.

To implement this dependency, we will:

1) Add a `ProvideFile` variant to `Dependency`.
2) Add a `add_file_provide_dependency` method to `Store`.
3) Add a `provide_file` method to `Context`.
4) Implement `provide_file` in `TopDownContext` (and `NonIncrementalContext`).

### Add `ProvideFile` variant to `Dependency`

### Add `add_file_provide_dependency` method to `Store`

### Add `provide_file` method to `Context`

### Implement `provide_file` in `TopDownContext`

### Implement `provide_file` in `NonIncrementalContext`
