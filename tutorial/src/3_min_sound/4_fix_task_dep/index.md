# Fix Superfluous Task Dependency

There is actually a massive bug in our implementation.
If you noticed the issue while following this tutorial, you're pretty observant!
If you didn't catch it, don't worry. 
I actually didn't catch it either up until this point!

I decided to keep the bug in to show how hard it is to test incrementality and soundness.

Let's start by adding another testing task which we need to uncover the bug, and then write a test that manifests the bug.

## Add `ToUpper` task

Modify `pie/src/tests/common/mod.rs` to add another task:

```diff2html fromfile linebyline
../../gen/3_min_sound/4_fix_task_dep/a_upper_task.rs.diff
```

The `ToUpper` task does (as expected) the opposite of the `ToLower` task: it requires the string providing task and returns the string in uppercase.

## Test case setup

No we set up a test case uncover the bug.
Modify `pie/src/tests/top_down.rs` to add another test:

```diff2html fromfile linebyline
../../gen/3_min_sound/4_fix_task_dep/b_test_setup.rs.diff
```

This test is similar to the previous one, but we have added a `ToUpper` task which requires the `ToLower` task.
We first require `ToLower` and assert that only `ToLower` and `ReadFile` are executed.
`ToUpper` should not be executed because we have not required it, and neither `ToLower` nor `ReadFile` require it.

Then, we require `ToUpper` and assert that it is executed.
Neither `ToLower` nor `ReadFile` should be executed because their dependencies are still consistent.

Check that this test, so far, succeeds with `cargo test`.
You can inspect the build log with `cargo test --test top_down test_no_superfluous_task_dependencies` to see what is going on, but it should look pretty normal.
The important part of this setup is that `ToLower` returns `"hello, world!"`.

## Manifest the bug

Manifest the bug by modifying `pie/src/tests/top_down.rs`:
                    
```diff2html fromfile linebyline
../../gen/3_min_sound/4_fix_task_dep/c_test_manifest.rs.diff
```

We change `file` in a very specific way: we capitalize the `l` characters to `L` characters.
We do this to trigger early cutoff.
By changing `file` in this way, we expect `ReadFile` to execute and return `"HeLLo, WorLd!"`.
This in turn means that `ToLower`'s task dependency to `ReadFile` is inconsistent, because the output changed, so `ToLower` is executed.
However, `ToLower` changes those `L` characters back into `l` and returns `"hello, world!"`, which is the same as last time.
Therefore, `ToUpper`'s task dependency to `ToLower` is still consistent, and we can cut off the build early.
We assert this inside the `require_then_assert` block.

But, if you run the tests with `cargo test`, this test will fail!
How can that be?

```admonish failure title="Expected Test Failure"
Test `test_no_superfluous_task_dependencies` will fail as expected, which we will fix in this section!
```

Inspect the build log with `cargo test --test top_down test_no_superfluous_task_dependencies`.
The third (last) build log should look like this:

```
{{#include ../../gen/3_min_sound/4_fix_task_dep/c_test_manifest_3.txt:2:}}
```

In this last build, `ToUpper` is required, and it will check its dependency to `ReadFile`.
But that shouldn't happen, because `ToUpper` only has a dependency to `ToLower`!
There seems to be a bug where `ToLower`'s task dependency to `ReadFile`, somehow ended up with `ToUpper`.

We need to go back to our consistency checking code to find the cause.

## Finding the cause

In the previous chapter, we implemented dynamic dependencies including an `is_inconsistent` method to check if a dependency is consistent.
This is the code we used for task dependencies:

```rust,
{{#include ../../2_incrementality/3_dependency/c_task.rs:30:38}}
```

To check if a task dependency is consistent, we call `require` on the context (which calls `require_task_with_stamper` with a default stamper).
Later on we implemented this `require_task_with_stamper` method for `TopDownContext`:

```rust,
{{#include ../2_tracker/e_top_down_tracker.rs:40:75}}
```

In the `if let Some(current_executing_task_node)` block we are adding a task dependency from the current executing task (if any), to the task being required.
This is the cause of the bug.
Even if we are only _consistency checking_ a task to see if it should be executed, we could end up adding a task dependency to the current executing task, which is not correct.
We only manifested the bug in the last test due to having a chain of 2 task dependencies, and by carefully controlling what is being executed and what is being checked.

Recall the second build in the `test_no_superfluous_task_dependencies` test.
The build log for that build looks like:

```
{{#include ../../gen/3_min_sound/4_fix_task_dep/c_test_manifest_2.txt:2:}}
```

In this build we are executing `ToUpper`, then require `ToLower`, then _consistency check_ `ReadFile`, which in turn _requires_ `ReadFile`.
At the end of that require, an incorrect dependency from `ToUpper` to `ReadFile` is made (although not really visible in the log).
This incorrect dependency then later breaks incrementality.

To fix this bug, we need to make sure that we only add task dependencies when an executing task directly requires another task, not when consistency checking!

## Fixing the bug

To fix the bug, we will separate the process of _making a task consistent_, which does not add task dependencies, from _requiring a task_, which can add task dependencies.
We will split this part into a `make_task_consistent` method.
Modify the top-down context from `pie/src/context/top_down.rs`:

```diff2html fromfile
../../gen/3_min_sound/4_fix_task_dep/d_1_make_consistent.rs.diff
```

We extracted the core of the `require_task_with_stamper` method into a `make_task_consistent` method, and call `make_task_consistent` in `require_task_with_stamper`.
This is a refactoring, so the `require_task_with_stamper` method will behave the same as before.

To reiterate, `make_task_consistent` makes given task is consistent by checking the task, executing it if inconsistent, and returning its output.
If it is already consistent, we return the cached output.
In both cases we also use and update `self.session.consistent`: the set of already consistent tasks this session. 

Now we need to change the `is_inconsistent` methods on dependencies to use `make_task_consistent` instead.
However, the `is_inconsistent` method is generic over `Context`, which doesn't expose the `make_task_consistent` method.
We also do not want to expose `make_task_consistent` to users of the library, as it would allow tasks authors to make tasks consistent without adding dependencies, which could break incrementality.
Therefore, we will define a `MakeConsistent` trait in the dependency module, and have `TopDownContext` implement that.

Modify `pie/src/dependency.rs`:

```diff2html fromfile
../../gen/3_min_sound/4_fix_task_dep/d_2_task_dependency.rs.diff
```

We add the `MakeConsistent` trait and use it in `is_inconsistent`. 
Now we go back to `TopDownContext` and implement that trait.

Modify `pie/src/context/top_down.rs`:

```diff2html fromfile
../../gen/3_min_sound/4_fix_task_dep/d_3_impl.rs.diff
```

We implement the `MakeConsistent` trait on `TopDownContext`, forwarding it to the `make_task_consistent` method.

We also need to implement this trait for the `NonIncrementalContext`.
Modify `pie/src/context/non_incremental.rs`:

```diff2html fromfile
../../gen/3_min_sound/4_fix_task_dep/d_4_non_incremental.rs.diff
```

If you run `cargo test` now, `test_no_superfluous_task_dependencies` should succeed, indicating that we fixed the bug!
However, `test_require_task` now fails 😅.

```admonish success title="Fixed Tests"
Test `test_no_superfluous_task_dependencies` should now succeed.
```

```admonish failure title="Expected Test Failure"
Test `test_require_task` will fail as expected, which we will now fix!
```

An `assert!(execute.start() > require.start())` in this test now fails, which is a sanity check asserting that "executes starts" should be later than "require starts"
This is because our changes have correctly removed several superfluous task requires, which influences these assertions.

Inspect the build log for this test with `cargo test --test top_down test_require_task`.
The second build now looks like:

```
{{#include ../../gen/3_min_sound/4_fix_task_dep/e_fix_tests_2.txt:2:}}
```

In this second build, `ReadFile` is now no longer required, and instead is only checked.
This is correct, and does not make any assertions fail.

The third build now looks like:

```
{{#include ../../gen/3_min_sound/4_fix_task_dep/e_fix_tests_3.txt:2:}}
```

In this third build, `ReadFile` is also no longer required at the start, but later on in the build it _is_ required when `ToLower` is executed.
This is correct, as it is only _checked_ (using `make_task_consistent`) at the start, but _required_ later while `ToLower` is executing.

The problem is that this assertion just does not hold anymore, as a task can be executed without first being required.
What does hold, is that a task is only executed after being checked _or_ required.
However, we don't track checking in the event tracker, so we will just remove this assertion to keep the tutorial going.
We will also update the expected build logs in the comments to reflect our changes.

Fix the tests by modifying `pie/src/tests/top_down.rs`:
                    
```diff2html fromfile
../../gen/3_min_sound/4_fix_task_dep/e_fix_tests.rs.diff
```

Confirm this fixes the tests with `cargo test`.
All tests are green! 🎉🎉🎉

```admonish success title="Fixed Tests"
Test `test_require_task` should now succeed.
```

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/4_fix_task_dep/source.zip).
```
