# Integration Testing

## Testing utilities

First we start by adding testing utilities (it never ends, does it?) that will make writing integration tests more convenient.
Unfortunately, we can't use `dev_shared` for this, as we would need to add a dependency to from `dev_shared` to `pie`, resulting in a dependency cycle because `pie` depends on `dev_shared`.

```admonish note title="Development Dependency Cycle" collapsible=true
If you would create this cycle, the code would still compile, but there would be 2 different instances of `pie` at the same time: one with unit testing enabled (`#[cfg(test)]`), and one without.
Even though these libraries are very similar, they are effectively 2 completely different libraries.
When `pie` uses code from `dev_shared` that depends again on `pie`, then there will be errors about types and traits not matching.

This is [probably a bug in cargo](https://github.com/rust-lang/cargo/issues/6765), or at least undesired behaviour. 
It should allow this cycle and make it work correctly, or disallow it.
```

We will put the utilities in a common file and use that as a module in integration tests.
Create the `pie/src/tests` directory, create the `pie/src/tests/common` directory, and create the `pie/src/tests/common/mod.rs` file.
Add the following code to `pie/src/tests/common/mod.rs`:

```rust,
{{#include a_1_common_pie.rs}}
```

These are just types and functions to create `TestPie` instances, which are `Pie` instances using `CompositeTracker<EventTracker, WritingTracker>` as tracker, where the writing tracker will write to standard output.

Add the following to `pie/src/tests/common/mod.rs`:

```rust,
{{#include a_2_common_ext.rs:2:}}
```

We define an extension trait `TestPieExt` with a `require_then_assert` method, which requires a task in a new session, asserts that there are no dependency check errors, and then gives us the opportunity to perform additional assertions via a function that gives access to `EventTracker`.
This is very convenient for integration testing, as most tests will follow the pattern of requiring a task and then asserting properties.

This trait also provides:

- `require` which is `require_then_assert` without an assertion closure,
- `require_then_assert_no_execute` which after requiring asserts that the task has not been executed using `!t.any_execution_of(task)` from `EventTracker`,
- `require_then_assert_one_execute` which does the same but asserts that it has been executed exactly once.

We implement `TestPieExt` for `TestPie` so that we can call `require_then_assert` on any `TestPie` instance.

```admonish tip title="Rust Help: Extension Trait" collapsible=true
Rust does not allow adding methods to an existing type/trait to ensure forward compatibility.
For example, if your library could add a method `foo` to `String`, but in a later Rust version the `String::foo` method would be added to the standard library, then all users of your library will run into an ambiguity and fail to compile.

Extension traits are a pattern in Rust where we can add methods to an existing type via a trait (typically named `TraitExt`) and an implementation of that trait for the existing type.
Because the extension trait must be imported to make the methods available to the current module, this can only cause compatibility issues if the trait is actually imported.
```

We still need to define a task for testing.
Add the following to `pie/src/tests/common/mod.rs`:

```rust,
{{#include a_3_common_task.rs:2:}}
```

We define a `TestTask` enumeration containing all testing tasks, which for now is just a `Return` task that just returns its string, and implement `Task` for it.
The `Output` for `TestTask` is `Result<TestOutput, ErrorKind>` so that we can propagate IO errors in the future.

`TestOutput` enumerates all possible outputs for `TestTask`, which for now is just a `String`.
We implement `From<String>` for `TestOutput` so we can easily convert `String`s into `TestOutput`. 
`as_str` performs the opposite operation.

Check that the code compiles with `cargo test`.

## First integration test

Now we're ready to test incrementality and soundness of the top-down incremental context through integration tests.
Create the `pie/src/tests/top_down.rs` file and add to it:

```rust,
{{#include b_test_execute.rs}}
```

In this first `test_execution` test we are just making sure that new tasks are executed, assert that the order of operations is correct, and check the task output.
We use `require_then_assert` to require the task and then perform assertions through a closure.
We're using `tracker.slice()` to get a slice of all build events, and assert (using [`assert_matches!`](https://docs.rs/assert_matches/latest/assert_matches/macro.assert_matches.html) again) that the following operations happen in order:

- start requiring `task`,
- start executing `task`,
- done executing `task`,
- done requiring `task`.

`require_then_assert` returns the output of the task, which is a `Result`, so we first propagate the error with `?`.
Finally, we assert that the output equals what we expect.

Check that this test succeeds with `cargo test`.
To see what test failures look like, temporarily change `events.get(2)` to `events.get(3)` for example.

```admonish tip title="Rust Help: Integration Testing" collapsible=true
[Integration tests](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html) in Rust are for testing whether the different parts of your library work together correctly.
Integration tests have access to the public API of your crate.

In this `top_down.rs` integration test file, we're importing `common/mod.rs` by creating a module for it via `mod common;`.
If we create another integration testing file, we would again create a module for it in that integration testing file.
This is because every file in the `tests` directory is compiled as a separate crate, and can basically be seen as a separate `lib.rs` or `main.rs` file.

Putting the testing utilities behind a `common` directory ensures that it will not be compiled as a separate integration testing crate. 
```

## Testing incrementality and soundness

We will now test incrementality and soundness.

### No dependencies

Let's first test that requiring a task without dependencies twice, only executes it once.
Add the following test to `pie/src/tests/top_down.rs`:

```rust,
{{#include c_test_reuse.rs:2:}}
```

We're using `require` and `require_then_assert_no_execute` from `TestPieExt` which require the same task twice, in two different sessions.
Since `Return` has no dependencies, it should only ever be executed once, after which its output is cached for all eternity.

Check that this test succeeds with `cargo test`.

~~~admonish tip title="Rust Help: Reading Standard Output from Tests"
Cargo runs tests in parallel by default, which is good to run all tests as fast as possible (and it's also safe due to Rust's memory-safety and thread-safety guarantees!)
However, this mixes the standard outputs of all tests, which makes reading the build log from our writing tracker impossible.
If you want to see the standard output, either:

- Run tests [consecutively](https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively) with: `cargo test -- --test-threads=1`
- Run a [single test](https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-single-tests) in the `top_down` integration test file with: `cargo test --test top_down test_reuse`

The second command should result in something like:

```
{{#include ../../gen/3_min_sound/3_test/c_test_reuse_stdout.txt}}
```
~~~

### Testing file dependencies

Next we want to test that a task with dependencies is not executed if its dependencies are consistent, and is executed when any of its dependencies are inconsistent.
Therefore, we need to add a task that has dependencies.

Modify `pie/src/tests/common/mod.rs`:

```diff2html fromfile
../../gen/3_min_sound/3_test/d_1_read_task.rs.diff
```

We add a `ReadFile` task that requires a file and returns its content as a string, similar to the ones we have implemented in the past.

Modify `pie/src/tests/top_down.rs` to add a new test:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/d_2_test_require_file.rs.diff
```

In `test_require_file`, we first create a temporary directory and `file`, and a `ReadFile` `task` that reads from `file`.
We require the `task` task several times, and assert whether it should be executed or not:

1) The task is new, so it should be executed, which we assert with `require_then_assert_one_execute`.
2) The task is not new, but its single require file dependency is still consistent, so it should not be executed.
3) We change the file the task depends on with `write_until_modified`.
4) We require the task again. This time it should be executed because its file dependency became inconsistent.

We repeat the test with the `FileStamper::Exists` stamper, which correctly results in the task only being executed once.
It is a new task because its stamper is different, and it is not re-executed when the file is changed due to `FileStamper::Exists` only checking if the file exists.

Note that in general, the `FileStamper::Exists` stamper is not a good stamper to use with `ReadFile`, because it will only be re-executed when the file is added or removed.
But for testing purposes, this is fine. 

Check that this test succeeds with `cargo test`.

### Testing task dependencies

Now it's time to test the more complicated task dependencies.
For that, we'll implement a task that depends on another task.
Modify `pie/src/tests/common/mod.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_1_lower_task.rs.diff
```

We add a `ToLower` task that requires another task (stored as `Box<TestTask>`) to get a `String`, which it then converts to lower case.
We also add the `into_string` method to `TestOutput` for conveniently getting an owned `String` from a `TestOutput`.

```admonish tip title="Rust Help: Boxing to Prevent Cyclic Size Calculation" collapsible=true
We store the string providing task as `Box<TestTask>` in order to prevent cyclic size calculation, which would cause `TestTask` to have an undetermined size.
This is due to several reasons:
- In Rust, values are stored on the stack by default. To store something on the stack, Rust needs to know its size *at compile-time*.
- The size of an `enum` is the size of the largest variant.
- The size of a struct is the sum of the size of the fields.

If we don't box the task, to calculate the size of the `ToLower` enum variant, we need to calculate the size of `TestTask`, which would require calculating the size of the `ToLower` variant, and so forth.
Therefore, we can't calulate the size of `ToLower` nor `TestTask`, which is an error.

Boxing solves this because `Box<TestTask>` allocates a `TestTask` on the heap, and then creates a pointer to it.
Therefore, the size of `Box<TestTask>` is the size of one pointer, breaking the cycle in the size calculations.

Note that this explanation [simplifies many aspects of Rust's size calculation](https://doc.rust-lang.org/nomicon/exotic-sizes.html).
```

Now add a test to `pie/src/tests/top_down.rs`: 

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_2_test_require_task.rs.diff
```

In `test_require_task`, we create a `read` task that reads from `file`, and a `lower` task that requires `read`.
In this test, we want to test three properties:

1) When we require `lower` for the first time, it will require `read`, which will require `file`, `read` will return the contents of `file` as a string, and `lower` will turn that string into lowercase and return it. 
2) When we require `lower` when `file` has not been changed, no task is executed.
3) When we require `lower` when `file`'s contents _have changed_, then first `read` must be executed, and then `lower` must be executed with the output of `read`.

#### Initial require

Test the first property by adding the following code to `pie/src/tests/top_down.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_3_test_require_task.rs.diff
```

We require `lower` for the first time and then assert properties using `require_then_assert`.
The comment shows the expected build log.

Inside `require_then_assert` we will make extensive use of the indexes and ranges from our `EventTracker`, and use `assert_matches!` to ensure these indexes and ranges exist (i.e., return `Some`).
Ranges (`RangeInclusive`) are just the start and end indices of events, accessed with `.start()` and `.end()`.
Indices are numbers (`usize`) that we can compare using the standard `>` operator.
A higher index indicates that the event happened later.

We get the ranges for requiring and executing the `lower` and `read` tasks, asserting that they are both required and executed.
Then we perform some sanity checks in `assert_task_temporally_sound`:

- Require and execute end events should come after their start events.
- A task only starts being executed after it starts being required. If a task is executed without being required (and thus without being checked), we are breaking incrementality.
- A task must only finish being required after it is finished being executed. If requiring a task ends before executing it, we are breaking soundness, because we are returning an inconsistent value to the requiring task.

We confirm that `file` is required and get the corresponding event index into `file_require`.
Then we assert several properties:

- `read` is required/executed while `lower` is being required/executed.
  - If `read` would be executed _after_ `lower` finished executing, we are breaking soundness because then we would have executed `lower` without first requiring/executing its dependencies.
  - If `read` would be executed _before_ `lower` started executing, we are breaking incrementality due to executing a task that was not required. In this test, we would not really break incrementality if this happened, but in general we could.
- `file` is required while `read` is being executed. A sanity check to ensure the file dependency is made by the right task.

Finally, we assert that the final output of requiring `lower` is `"hello world!"`, which is the contents of the file in lowercase.
Check that this test succeeds with `cargo test`.
That concludes the first property that we wanted to test!

#### No changes

The second one is easier: when `file` has not changed, no task is executed.
Add the following code to `pie/src/tests/top_down.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_4_test_require_task.rs.diff
```

Here we change nothing and use `require_then_assert_no_execute` to assert no task is executed.
Check that this test succeeds with `cargo test`.

#### Changed file affects task

Now we test the third property, testing soundness and incrementality after a change.
Add the following code to `pie/src/tests/top_down.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_5_test_require_task.rs.diff
```

We first change the `file` contents such that `read`'s `file` dependency becomes inconsistent, and then require `lower` again.
In the `require_then_assert` block we first assert that both tasks are required and executed, `file` is required, and perform sanity checks again.

Now let's go back to the build log in the comment, which is lot more complicated this time due to recursive consistency checking. The gist is:

- To check if `lower` should be executed, we check its dependencies: a task dependency to `read`.
  - To check if `read` should be executed, we check its dependencies: a `file` dependency, which is inconsistent, thus we execute `read`.
  - `read` executes and now returns `"!DLROW OLLEH"` instead of `"HELLO WORLD!"`.
- Then we are back to checking `lower`'s task dependency to `read`, which is inconsistent because `read` returns a different value, which is inconsistent due to the equals output stamper. 
- Thus, we execute `lower` which requires `read`.
- We can skip checking `read` because we already checked and executed it: it is deemed consistent this session. We immediately return its output `"!DLROW OLLEH"` to `lower`.
- `lower` turns the string lowercase and returns it.

Note that we are executing `read` _before_ executing `lower` this time (but still _while requiring_ `lower`).
This is important for incrementality because if `read` had not returned a different output, we would not have to execute `lower` due to its equals output stamp still being consistent (we call this _early cutoff_).
We test this property with the last 3 assertions in the `require_then_assert` block.

Finally, we assert that the output is `"!dlrow olleh"` as expected.
Confirm that this test succeeds with `cargo test`.

Now that we're testing task dependencies anyway, let's also test a fourth property: the early cutoff behaviour.

#### Early cutoff

Early cutoff can happen in this test when `read` is re-executed due to its file dependency being inconsistent (modified file stamp change), but returns the same output as last time.
In that case, we don't have to execute `lower` because its task dependency to `read` is still consistent (equals output stamp is the same).
We can trigger this case in this test by changing `file` such that its last modified date changes, but its contents stay the same.

Add the following code to `pie/src/tests/top_down.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/e_6_test_require_task.rs.diff
```

We change `file` in the way we discussed, and then assert that `read` is executed, but `lower` is not.
Confirm that this test succeeds with `cargo test`.

```admonish info title="Benefits of Early Cutoff"
Early cutoff is one of the great benefits of a build system with precise dynamic dependencies.
In larger builds, it can cut off large parts of the build which do not need to be executed.

In our build system, we only have the simple equals output stamper.
But if you extend the build system with user-defined stampers (which isn't too hard), task authors have much more control over early cutoff.
For example, we could require a task that parses a configuration file, but use a stamper that extracts only the particular configuration option our task is using.
Then, our task will only be re-executed if that configuration option changes.

Thus, stampers can increase the precision of task dependencies, which in turn increases incrementality with early cutoff.
```

Nice! These tests give quite some confidence that what we've been doing so far seems to be sound and incremental.
We can (and should) of course write more tests for better coverage of the implementation.
For example, we haven't tested tasks with multiple dependencies yet.
However, in this tutorial we will move on to a couple of specific tests first, because there are several issues still hiding in our implementation: (at least) one bug, and three soundness holes.
After we've uncovered those issues and fix them, feel free to write more tests yourself!

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/3_test/source.zip).
```
