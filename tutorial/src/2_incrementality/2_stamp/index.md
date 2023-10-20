# Stamps

To check whether we need to execute a task, we need to check the dependencies of that task to see if any of them are inconsistent.
To make this consistency checking configurable, we will use stamps.
A dependency is inconsistent if after stamping, the new stamp is different from the old stamp.
Therefore, we will implement a `FileStamper` that stamps files and produces a `FileStamp`, and an `OutputStamper` that stamps task outputs and produces an `OutputStamp`.

Add the `stamp` module to `pie/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/2_stamp/a_module.rs.diff
```

This module is public as users of the library will construct stampers.

## File stamps

Create the `pie/src/stamp.rs` file and add:

```rust,
{{#include b_file.rs}}
```

We're implementing `FileStamper` as an enum for simplicity.

A `FileStamper` has a single method `stamp` which takes something that can be dereferenced to a path, and produces a `FileStamp` or an error if creating the stamp failed.
For now, we implement only two kinds of file stampers: `Exists` and `Modified`.
The `Exists` stamper just returns a boolean indicating whether a file exists.
It can be used to create a file dependency where a task behaves differently based on whether a file exists or not.
The `Modified` stamper returns the last modification date if the file exists, or `None` if the file does not exist.

We derive `Eq` for stamps so that we can compare them.
Equal (same) stamps indicate a consistent dependency, unequal (different) indicates inconsistent.
We also derive `Eq` for stampers, because the stamper of a dependency could change, making the dependency inconsistent.

## Task output stamps

We implement task output stampers in a similar way.
Add to `pie/src/stamp.rs`:

```rust,
{{#include c_output.rs:3:}}
```

The `Inconsequential` stamper simply ignores the output and always returns the same stamp (thus is always equal).
It can be used to create a task dependency where we are interested in some side effect of a task, but don't care about its output.
The `Equals` stamper simply wraps the output of a task, so the stamp is equal when the output is equal.

Output stamps are generic over the task output type `O`.

```admonish tip title="Trait Bounds and Derive Macros" collapsible=true
Because `O` is used in the enum, the `derive` attributes on `OutputStamp` create bounds over `O`.
Thus, `OutputStamp` is only `Clone` when `O` is `Clone`, `OutputStamp` is only `Eq` when `O` is `Eq`, and so forth.
Because we declared `Task::Output` with bound `Clone + Eq + Debug`, we can be sure that `OutputStamp` is always `Clone`, `Eq`, and `Debug`.
```

```admonish question title="User-Defined Stamps?" collapsible=true
`FileStamper` and `OutputStamper` could also be a trait which would allow users of the library to implement their own stampers.
For simplicity, we do not explore that option in this tutorial.
In the actual PIE library, stampers (called checkers) can be implemented by users of the library!
```

## Tests

Finally, we write some tests.
Add to `pie/src/stamp.rs`:

```rust,
{{#include d1_test.rs:3:}}
```

We test file stamps by creating a stamp, changing the file, creating a new stamp, and then compare the stamps.
We test task output stamps by just passing a different output value to the `stamp` function, and then compare the stamps.

Run `cargo test` to test the stamp implementation.
However, a test could fail on some operating systems.
Do continue to the next subsection if this happens.

```admonish warning title="Likely Test Failure"
Test `test_modified_file_stamper` will likely fail. Do continue to the next subsection, because we're going to fix it!
```

## Testing with file modified time, correctly

Unfortunately, these tests may fail on some operating systems (Linux and Windows in my testing), due to an imprecise file last modified timer.
What can happen is that we write to a file, making the OS update its modified time to `1000` (as an example, not a real timestamp), then very quickly write to the file again, making the OS update its modified time to `1000` again.
Then, our test will fail because the stamp didn't change even though we expect it to change.

This can happen with an imprecise timer that only increases once every millisecond (again, an example, not a real number) when we perform writes in between that millisecond.
Even worse, our test can be flaky, sometimes succeeding if we write in between those milliseconds, sometimes failing if we write within a millisecond.

To solve this, add a function to the filesystem testing utility crate.
Change `dev_shared/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/2_stamp/d2_test_utilities.rs.diff
```

The `write_until_modified` function writes to the file, but ensures its modified time will change.
Now change the tests in `pie/src/stamp.rs` to use this function:

```diff2html fromfile
../../gen/2_incrementality/2_stamp/d3_test_correct.rs.diff
```

Now we use `write_until_modified` to write to the file, ensuring its modified time will change, ensuring the stamp will change when it should.
Run `cargo test` to confirm the stamp implementation, which should succeed now.

```admonish success title="Fixed Tests"
Test `test_modified_file_stamper` should now succeed.
```

## Stamps in Context

We now have a module dedicated to stamps.
However, stampers are constructed by users of the library that author tasks, and they need to pass in these stampers when creating dependencies.
Therefore, we need to update the `Context` trait to allow passing in these stampers.

Change `Context` in `pie/src/lib.rs`:

```diff2html fromfile
../../gen/2_incrementality/2_stamp/e_context_file.rs.diff
```

We add the `require_file_with_stamper` method which allow passing in a stamper.
We add a default implementation for `require_file` that passes in a default stamper.
The default is provided by `default_require_file_stamper` which can be overridden by context implementations.

Now apply the same to tasks, changing `Context` again in `pie/src/lib.rs`:

```diff2html fromfile
../../gen/2_incrementality/2_stamp/f_context_task.rs.diff
```

Update `NonIncrementalContext` in `src/context/non_incremental.rs` to implement the new methods:

```diff2html fromfile
../../gen/2_incrementality/2_stamp/g_non_incremental_context.rs.diff
```

We just ignore the stampers in `NonIncrementalContext`, as they are only needed for incrementality.

Run `cargo test` to confirm everything still works.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/2_stamp/source.zip).
```
