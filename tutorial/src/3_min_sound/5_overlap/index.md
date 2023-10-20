# Prevent Overlapping File Writes

So far we have only considered reading files in the build system.
However, there are many tasks that also write files.
For example, a C compiler reads a C source file (and header files) and writes an object file with the compiled result, which is typically an intermediate file that gets passed to a linker.
Another example is a file copy task that reads a file and copies it to another file.

We can handle file writes in tasks with `context.require_file`.
However, what should happen when two tasks write to the same file?
In a non-incremental setting, the last writer wins by overwriting (or appending to) the file.
Does this behaviour also occur in our incremental build system?

Unfortunately, this is not always the case in our incremental build system, because we can `require` individual tasks in a specific order that would cause an inconsistency, making the first writer win.
This is a bit tricky to explain without an example, so we will first add some testing tasks and write a test that showcases the problem.
In this section, we will continue with:
 
1) Add the `WriteFile` and `Sequence` tasks to the testing tasks.
2) Create a `test_overlapping_file_write` test to showcase the issue.
3) Introduce a new kind of dependency: a _provide file dependency_ for writing to (and creating) files.
4) Prevent overlapping file writes by checking for them at runtime, fixing the issue.
5) Improve and add additional tests

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
When we `require` a task, we are asking the build system to make **that task** consistent, and get its up-to-date output.
We are **not** asking the build system to make **all** tasks consistent.
The build system recursively checks and executes only the tasks that are absolutely necessary to make that task consistent.
If it would not do that, it would not truly be incremental!
Therefore, we cannot (and shouldn't) get rid of this feature, and instead need to find another solution to this problem.

While we require tasks "manually" here, through the `Pie` / `Session` API, this problem can also occur with tasks that require other tasks.
For example, if `seq` would just be `Sequence(vec![write_1])`, and we'd end up in the same inconsistent state when requiring `seq`.
Especially in large incremental builds with many different tasks, this can easily occur accidentally, causing subtle incrementality bugs.

Let's go back to the test.
In the test, `output_file` is not in a consistent state because `write_2` is inconsistent and needs to be executed to bring `output_file` into a consistent state.
However, if `write_2` would write to another file, there would be no inconsistency.
Let's write a test with separate output files.

Add the following test to `pie/tests/top_down.rs`:

```rust,
{{#include c_test_separate.rs:2:}}
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
2) Update `Tracker` and implementations to handle file provide dependencies.
3) Add a `add_file_provide_dependency` method to `Store`.
4) Add `provide_file` methods to `Context` and implement them in implementations.

### Add `ProvideFile` variant to `Dependency`

Modify `pie/src/dependency.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/d_dependency.rs.diff}}
```

We add the `ProvideFile` variant to `Dependency`, handle it in `is_inconsistent`, and update the unit test to also test that variant.
If you compile the code, you'll get an error because this new variant needs to be handled in `WritingTracker`, so let's update the trackers first.

### Update `Tracker` and implementations

Update `pie/src/tracker/mod.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/e_1_tracker.rs.diff}}
```

We add `provide_file_end` to `Tracker` and handle it in `CompositeTracker`.

Update `pie/src/tracker/writing.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/e_2_writing.rs.diff}}
```

We change `require_file_end` to print `r` instead of `-` to distinguish it from file provides.
We implement the `provide_file_end` method, printing the provided file.
Finally, we support the `Dependency::ProvideFile` variant by adding a branch for it in the match statement.

This fixes the compilation error.
Check that everything works with `cargo test`.

### Add `add_file_provide_dependency` method to `Store`

First we need to support provide file dependencies in the `Store`.
Update `pie/src/store.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/f_store.rs.diff}}
```

We add the `add_file_provide_dependency` method, which does the same as `add_require_provide_dependency` but creates a `ProvideFile` dependency instead.
We update the `test_dependencies` unit test to create a provide file dependency, and add a test to check whether `add_require_provide_dependency` panics when used incorrectly.
Confirm the changes work with `cargo test`.

### Add methods to `Context` and implementations

We are not creating provide file dependencies yet, so let's work on that now.
Add methods to `Context`, enabling tasks to create provide file dependencies, in `pie/src/lib.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/g_context.rs.diff}}
```

These methods are like `require_file`, but must be called **after** writing to the file, so that the stamper creates a stamp that includes the (meta)data that was written to the file.
Therefore, these methods do not return a `File` handle, because the caller creates a file handle for writing.

Implement this method in `pie/src/context/non_incremental.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/h_non_incr.rs.diff}}
```

The non-incremental context just ignores provided files.

Implement the method in `pie/src/context/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/i_top_down.rs.diff}}
```

Again, this method is similar to the requiring version, except that it creates a provide file dependency, and returns `()` instead of a file handle.
Check that your changes work with `cargo test`.

## Detect and disallow overlapping provided files

Now we will detect and disallow overlapping provided files.
The only source of provided files is the `provide_file_with_stamper` method we just implemented.
Therefore, we can easily check for overlapping dependencies there.
Whenever a file provide dependency is made, we just need to check if a task is already providing that file, and disallow that.

First add a method to `Store` to get the providing task (if any) for a file.
Modify `pie/src/store.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/j_1_store.rs.diff}}
```

`get_task_providing_file` does exactly that.
We get an iterator over incoming dependency edges for the destination file node using `get_incoming_edges`.
We use [`filter_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter_map) to both filter out non-provide file dependencies, and map to a `TaskNode` when it is a provide file dependency.
Only tasks can be the source of provide file dependencies, so it is always correct to create a `TaskNode` here.
We get the first (if any) element of the iterator with `.next()`, which is the task that provides the file.
Because we will disallow multiple tasks from providing a file, this method will return `Option<TaskNode>`, since there can only be 0 or 1 task providing a file.

We modify the dependency test again, testing that `get_task_providing_file` returns what we expect.

We assert (in development builds, like `get_dependencies_of_task`) that the file node must exist in the dependency graph, as a sanity check, and test that. 

Now we'll use this in `provide_file_with_stamper` to panic when overlap is detected.
Modify `pie/src/context/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/j_2_top_down.rs.diff}}
```

If we find a task that provides the file, we panic with a message explaining why.
Note that we don't have to check whether `previous_providing_task_node == current_executing_task_node`, and then not panic.
This is because when executing a task, we first reset it, which removes all its outgoing dependencies.
Therefore, `current_executing_task_node` cannot have a provide file dependency to the file.
Unless it provides the same file twice, but that is overlap that we also want to disallow.

```admonish question title="Why Panic?"
We discussed panicking in the section on the incremental top-down context, but want to reiterate it here.
Instead of panicking, we could have `provide_file_with_stamper` return an error indicating overlap was found.
However, that error would then propagate throughout the entire API.
Tasks would have to propagate it in their `execute` method, meaning that `Context::require` will also be able to return this error.
When tasks already return their own errors, you'd end up with return types such as `Result<Result<AnOutput, AnError>, OverlapError>` which are annoying to deal with.

This is a hard trade-off to make, but in this tutorial (and in the actual PIE library) we will panic.
```

Confirm our changes work with `cargo test`.
Wait, shouldn't the overlap test now fail?
No, we didn't change our `WriteFile` task to use `provide_file` yet.
Let's fix that now.

Modify `pie/tests/common/mod.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/k_1_use_provide.rs.diff}}
```

Just replace `require_file_with_stamper` with `provide_file_with_stamper` in `WriteFile`.
Running `cargo test` should make the overlap test now fail!

```admonish failure title="Expected Test Failure"
Test `test_show_overlap_issue` will fail as expected, which we will now fix!
```

Modify the test in `pie/tests/top_down.rs`:

```diff2html
{{#include ../../gen/3_min_sound/5_overlap/k_2_fix_test.rs.diff}}
```

We change the test into one that should panic.
We use `expected = "Overlapping provided file"` to indicate that the panic should include `"Overlapping provided file"`, so that the test does not succeed due to another unrelated panic.

Unfortunately, tests that should panic may not return a `Result`.
We work around that by wrapping the entire test in a nested `run` function that returns a `Result`, and by calling `run().unwrap()` at the end of the test.

We rename the test to `test_overlapping_provided_file_panics` which better describes what it is testing and what is expected.
And we simply the test a lot, because it will panic when we call `require`, so the other part of the test is no longer required. 

Run `cargo test` to check that this test will now succeed.

```admonish success title="Fixed Tests"
Test `test_overlapping_provided_file_panics` (was: `test_show_overlap_issue`) should now succeed.
```

Let's add two more tests: one that confirms overlap is detected when we manually `require` two different tasks, and one that confirms that requiring (and executing) the same task does not cause overlap.
Add these tests to `pie/tests/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/5_overlap/k_3_more_tests.rs.diff}}
```

Confirm that these tests also succeed with `cargo test`.

We're now preventing the inconsistencies of overlapping file writes that occur in an incremental setting.
This does require some care when writing to files in a programmatic incremental build system, as task authors need to ensure that distinct tasks only write to distinct files.
And we only detect this at run-time, while running the build, so task authors must test their tasks, combinations of tasks, and with different inputs, to have some certainty that their tasks have no overlapping file writes.
However, I think this kind of run-time checking is preferable over incremental builds being inconsistent or incorrect. 

```admonish question title="Detect Overlap Statically?" collapsible=true
As far as I know, there is no _easy_ way to detect overlap statically in the presence of dynamic dependencies and incrementality.
You'd have to encode file names and paths in the type system, and restrict what kind of names and paths you can use.

Matthew Hammer et al. developed [Fungi, a typed functional language for incremental computation with names](https://arxiv.org/abs/1808.07826) to solve these kind of problems, but it is quite involved!
Be sure to read that paper and their previous work on [Adapton (non-HTTPS)](http://adapton.org/) if you're interested in that line of research. 
```

In the next section, we will detect and disallow another inconsistency in incremental builds: hidden dependencies.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/5_overlap/source.zip).
```
