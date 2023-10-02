# Integration Testing

## Testing utilities

First we start by adding testing utilities (it never ends, does it?) that will make writing integration tests more convenient.
Unfortunately, we can't use `dev_shared` for this, as we would need to add a dependency to from `dev_shared` to `pie`, resulting in a dependency cycle because `pie` depends on `dev_shared`.

```admonish info title="Development dependency cycle" collapsible=true
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

```admonish info title="Extension trait" collapsible=true
Extension traits are a pattern in Rust where we can add methods to an existing type via an extension trait and an implementation of the extension trait for the existing type.
```

We still need to define a task for testing.
Add the following to `pie/src/tests/common/mod.rs`:

```rust,
{{#include a_3_common_task.rs:2:}}
```

We define a `TestTask` enumeration containing all testing tasks, which for now is just a `StringConstant` task that returns a string, and implement `Task` for it.
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

```admonish info title="Integration testing in Rust" collapsible=true
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
Since `StringConstant` has no dependencies, it should only ever be executed once, after which its output is cached for all eternity.

Check that this test succeeds with `cargo test`.

~~~admonish info title="Reading standard output from tests"
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

### File dependency

Next we want to test that a task with dependencies is not executed if its dependencies are consistent, and is executed when any of its dependencies are inconsistent.
Therefore, we need to add a task that has dependencies.

Modify `pie/src/tests/common/mod.rs`:

```diff2html fromfile
../../gen/3_min_sound/3_test/d_1_read_task.rs.diff
```

We add a `ReadStringFromFile` task that requires a file and returns its content as a string, similar to the ones we have implemented in the past.

Modify `pie/src/tests/top_down.rs` to add a new test:

```diff2html fromfile linebyline
../../gen/3_min_sound/3_test/d_2_test_require_file.rs.diff
```

In this test, we require a `ReadStringFromFile` task several times, and assert whether it should be executed or not.
The first time, the task is new, so it should be executed.
The second time, the task is not new, and its single require file dependency is still consistent, so it should not be executed.
Then, we change the file the task depends on with `write_until_modified`.
Then, we require the task again, and it should be executed because its dependency became inconsistent.

We repeat the test with the `FileStamper::Exists` stamper, which correctly results in the task only being executed once.
It is a new task because its stamper is different, and it is not re-executed when the file is changed due to `FileStamper::Exists` only checking if the file exists.

Check that this test succeeds with `cargo test`.
