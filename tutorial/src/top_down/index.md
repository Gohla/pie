# Incremental Top-Down Context

In this chapter, we will implement an *incremental* build context.
An incremental context selectively executes tasks — only those that are affected by a change.
In other words, an incremental context executes the *minimum number of tasks* required to make all tasks up-to-date.

However, due to dynamic dependencies, this is not trivial.
We cannot first gather all tasks into a dependency tree and then topologically sort that, as dependencies are added and removed *while tasks are executing*.
To do incremental builds in the presence of dynamic dependencies, we need to check and execute affected tasks *one at a time*, updating the dependency graph, while tasks are executing.
To achieve this, we will employ a technique called *top-down incremental building*, which starts checking if a top (root) task needs to be executed, and recursively checks whether dependent tasks should be executed until we reach the bottom (leaf) task(s), akin to a depth-first search.

Furthermore, build systems almost always interact with the file system in some way. 
For example, tasks read configuration and source files, or write intermediate and binary files.
Thus, a change in a file can affect a task that reads it, and executing a task can result in writing to new or existing files.
Therefore, we will also keep track of *file dependencies*.
Like task dependencies, file dependencies are also tracked dynamically while tasks are executing.

Before we start coding, let's sketch the outline of the solution — we will:

* Extend `Context` with a way to for tasks to register file dependencies.
  * Implement file system utility functions in module `fs`.
  * Make `NonIncrementalContext` compatible with the extension to `Context`.
* Implement an `TopDownContext` that does incremental building.
  * Implement `Dependency` to represent dependencies.
  * Implement `Store` that keeps track of the dependency graph.
* Write tests for `TopDownContext` to confirm that it is sound and incremental.
  * Implement a `Tracker` that can track build events, so we can assert whether a task has executed or not to test incrementality.

## Adding File Dependencies

To support file dependencies, add a method to the `Context` trait in `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/0_lib_b.rs.diff:4:}}
```

`require_file` is similar to requiring a task, but instead takes a `path` to a file or directory on the filesystem as input.
We use `AsRef<Path>` as the type for the path, so that we can pass anything in that can dereference to a path.
For example, `str` has an `AsRef<Path>` implementation, so we can just use `"test.txt"` as a path.

As an output, we return `Result<Option<File>, io::Error>`, with `File` being a handle to an open file.
The reason for this complicated type is:

* An incremental context will want to read the metadata (such as the last modified date) of the file, or create a hash over the file, to be able to detect changes. Because getting metadata or reading the file can fail, and we want to propagate this error, we return a `Result` with `io::Error` as the error type.
* Tasks can create a dependency to a file that does not exist, and the existence of that file affects the task. For example, a task that prints true or false based on if a file exists. If the file does not exist (or it is a directory), we cannot open it, so we cannot return a `File`, hence we use `Option<File>` to return `None`.
* Otherwise, we return `Ok(Some(file))` so that the task can read the opened file.

```admonish info title="Rust Help" collapsible=true
[Recoverable error handling](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html) in Rust is done with the `Result<T, E>` type, which can either be `Ok(t)` or `Err(e)`.
In contrast to many languages which use exceptions, throwing, and exception handling; Rust treats recoverable errors just as regular values.

Similarly, [optional values](https://doc.rust-lang.org/std/option/) in Rust are defined using the `Option<T>` type, which can either be `Some(t)` or `None`.

Rust has many traits for converting values or references into others, which provides a lot of convenience in what would otherwise require a lot of explicit conversions.
[`AsRef<T>`](https://doc.rust-lang.org/std/convert/trait.AsRef.html) is such a conversion trait, that can convert itself into `&T`. 
Here, we use `AsRef<Path>` as a generic with a trait bound to support many different kinds of values to the `path` argument in `require_file`.
For example, we can call `context.require_file("test.txt")` because `str`, which is the type of string constants, [implements `AsRef<Path>`](https://doc.rust-lang.org/src/std/path.rs.html#3136-3141).
You can also see this as a kind of method overloading, without having to provide concrete overloads for all supported types.
```

Now we need to implement this method for `NonIncrementalContext`.
However, because we will be performing similar file system operations in the incremental context as well, we will create some utility functions for this first.



Add the `fs` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/0_lib_c.rs.diff:4:}}
```

Create file `src/fs.rs` with:

```rust,
{{#include 0_fs.rs}}
```

The comments explain the behaviour.

We will write some tests to confirm the behaviour, but for that we need a utility to create temporary files and directories.
Instead of implementing that ourselves, we will use an existing crate.
Add the `tempfile` dependency to `Cargo.toml`:

```toml,customdiff
{{#include ../../gen/top_down/0_Cargo.toml.diff:4:}}
```

Note that this is dependency is added under `dev-dependencies`, indicating that this dependency is only available when running tests, benchmarks, and examples.
Therefore, users of our library will not depend on this library, which is good because temporary file creation is not necessary to users of our library.

Now, add the following tests to `src/fs.rs`:

```rust,
{{#include 0_fs_test.rs}}
```

Unfortunately, we can't easily test when `metadata` and `open_if_file` should return an error, because we cannot disable read permissions on files via the Rust standard library.

Now we are done with our filesystem utility excursion.
Make the non-incremental context compatible by changing `src/context/non_incremental.rs`:

```rust,customdiff
{{#include ../../gen/top_down/0_non_incremental_context.rs.diff:4:}}
```

Since the non-incremental context does not track anything, we simply try to open the file and return it.
This implements the description we made earlier:

* If opening the file results in an error, the `?` operator returns `Err(...)` immediately.
* If the file does not exist or is a directory, `open_if_file` returns `None` and `file` is `None`.
* Otherwise, `file` is `Some(file)`.

Confirm everything works with `cargo test`.

```admonish info title="Rust Help" collapsible=true

The `?` operator makes it easy to [propgate errors](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator).
Because errors are just values in Rust, to propgate an error, you'd normally have to match each result and manually propagate the error.
The `r?` operator applied to a `Result` `r` does this for you, it basically desugars to something like `match r { Err(e) => return Err(e), _ => {} }`.

Comments with three forward slashes `///` are [documentation comments](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments) that document the function/struct/enum/trait/etc. they are applied to.
```

## Implementing the Incremental Context

Now we get to the fun part, incrementality!

To check whether we need to execute a task, we need to check the dependencies of that task to see if any of them are not consistent.
If they are all consistent, we just return the cached output of the task.
If not, we just execute the task.

To implement this, we will need 3 components:
- A `Dependency` type which holds dependency data with methods for checking consistency.
- A `Store` type which holds the dependency graph with methods for interacting with the graph.
- A `TopDownContext` type that implements `Context` and owns a `Store`.

We will start with implementing `Dependency`, as it can be implemented as a stand-alone part.

### Dependency implementation

Add the `dependency` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/1_dependency_module.rs.diff:4:}}
```

Then create the `src/dependency.rs` file and add:

```rust,
{{#include 1_dependency.rs}}
```

We implement the `TaskDependency` and `FileDependency` types to handle task and file dependencies respectively.
We merge those two kinds of dependencies into the `Dependency` enum.
This split is made so that users of this module can accept only task or file dependencies, or any dependency in general, which we will need in the future.

A task dependency is inconsistent if, after recursively checking it, its output has changed.
The `TaskDependency::is_inconsistent` does exactly that, by requiring the task with a context, and then checking if the output has changed.
We implement a `is_inconsistent` method here instead of an `is_consistent` method, because we will change it in the future to return the changed output for logging purposes, and in that case we want to see the changed output if it is not consistent.

A file dependency is inconsistent if its last modification date has changed, or if the file did not exist before but does now (and vice versa), implemented in `FileDependency::is_inconsistent`.
If a file does not exist, we use `None` as the modification date, which does not equal `Some(modified_date)`.
We deal with errors (`io::Error`) by propagating them.

The `FileDependency::new` function also returns the opened file if it exists, so that users of this function can read from the file without having to open it again.

Finally, `Dependency` just merges the two kinds of dependencies and provides an `is_inconsistent` method that calls the corresponding method.

As usual, we write some tests to confirm the behaviour. Add tests to `src/dependency.rs`:

```rust,
{{#include 1_dependency_test.rs}}
```
