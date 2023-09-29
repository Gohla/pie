# Incrementality Example

In this example, we will run our build system and show off simple incrementality with a task that reads a string from a file.

## `ReadStringFromFile` task

Create the `pie/examples` directory, and create the `pie/examples/incrementality.rs` file with the following contents:

```rust,
{{#include ../6_example/a_task.rs}}
``` 

The `ReadStringFromFile` task is similar to the one we defined earlier in a test, but this one accepts a `FileStamper` as input, and propagates errors by returning a `Result`.
We cannot use `std::io::Error` as the error in the `Result`, because it does not implement `Clone` nor `Eq`, which need to be implemented for task outputs.
Therefore, we use `std::io::ErrorKind` which does implement these traits.

## Exploring incrementality

We've implemented the task, now add a `main` function to `pie/examples/incrementality.rs`:

```rust,
{{#include ../6_example/b_main.rs:2:}}
```

We create a temporary file, create a task, create a context, and require our first task.
Run this example with `cargo run --example incremental`.
You should see the `println!` in `ReadStringFromFile` appear in your console as the incremental context correctly determines that this task is new (i.e., has no output) and must be executed.
It should look something like:

```
{{#include ../../gen/2_incrementality/6_example/b_main.txt}}
```

### Reuse

If we require the task again, what should happen?

Insert the following code into the `main` method:

```rust,
{{#include ../6_example/c_reuse.rs:1:}}
```

Running with `cargo run --example incremental` should produce output like:

```
{{#include ../../gen/2_incrementality/6_example/c_reuse.txt}}
```

We don't see the `println!` from `ReadStringFromFile`, so it was not executed, so our incremental build system has correctly reused its output!

Normally we would write a test to confirm that the task was executed the first time, and that it was not executed the second time.
However, this is not trivial.
How do we know if the task was executed?
We could track it with a global mutable boolean that `ReadStringFromFile` keeps track of, but this quickly becomes a mess.
Therefore, we will look into creating a proper testing infrastructure in the next chapter.

For now, we will continue this example with a couple more interesting cases.
The comments in the code explain in more detail why the build system behaves in this way.

### Inconsistent file dependency

Insert into the `main` method:

```rust,
{{#include ../6_example/d_file_dep.rs:2:}}
```

If we change the file (using `write_until_modified` to ensure that the modified time changes to trigger the `Modified` file stamper) and require the task, it should execute, because the file dependency of the task is no longer consistent.

### Different tasks

Insert into the `main` method:

```rust,
{{#include ../6_example/e_diff_task.rs:2:}}
```

The identity of tasks is determined by their `Eq` and `Hash` implementations, which are typically derived to compare and hash all their fields.
Therefore, if we create read tasks for different input file `input_file_b` and different stamper `FileStamper::Exists`, these read tasks are not equal to the existing read task, and thus are *new* tasks with a different identity.
We require `read_task_b_modified` and `read_task_b_exists`, they are new, and are therefore executed.

### Same file different stampers

Insert into the `main` method:

```rust,
{{#include ../6_example/f_diff_stamp.rs:2:}}
```

Here we write to `input_file_b` and then require `read_task_b_modified` and `read_task_b_exists`.
We expect `read_task_b_modified` to be executed, but `read_task_b_exists` to be skipped, because its file dependency only checks for the existence of the input file, which has not changed.
This shows that tasks can depend on the same file with different stampers, which influences whether the tasks are affected by a file change individually.

Of course, using an `Exists` stamper for `ReadStringFromFile` does not make a lot of sense, but this is for demonstration purposes only.

Running `cargo run --example incremental` now should produce output like:

```
{{#include ../../gen/2_incrementality/6_example/f_diff_stamp.txt}}
```

Feel free to experiment more with this example (or new example files) before continuing.
In the next chapter, we will define minimality and soundness, set up an infrastructure for testing those properties, and fix issues uncovered by testing.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/6_example/source.zip).
```
