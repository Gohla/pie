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

There are several ways to check if a file dependency is consistent (i.e., has not changed), such as checking the last modification date, or comparing a hash.
To make this configurable on a per-dependency basis, we will implement *stamps*.
A file stamp is just a value that is produced from a file, such as the modification date or hash, that is stored with the file dependency.
To check if a file dependency is consistent, we just stamp the file again and compare it with the stored stamp.

Similarly, we can employ stamps for task dependencies as well by stamping the output of a task.

Before we start coding, let's sketch the outline of the solution — we will:

* Extend `Context` with a way to for tasks to register file dependencies.
  * Implement file system utility functions in module `fs`.
  * Make `NonIncrementalContext` compatible with the extension to `Context`.
* Implement a `TopDownContext` that does incremental building.
  * Implement file and task output stamps.
  * Extend `Context` to support stampers when creating dependencies.
  * Implement file and task dependencies.
  * Implement `Store` that keeps track of the dependency graph.
* Write tests for `TopDownContext` to confirm that it is sound and incremental.
  * Implement a `Tracker` that can track build events, so we can assert whether a task has executed or not to test incrementality.

## Adding File Dependencies

To support file dependencies, add a method to the `Context` trait in `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/0_require_file/a_context.rs.diff:4:}}
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
{{#include ../../gen/top_down/0_require_file/b_fs_module.rs.diff:4:}}
```

Create file `src/fs.rs` with:

```rust,
{{#include 0_require_file/c_fs.rs}}
```

The comments explain the behaviour.

We will write some tests to confirm the behaviour, but for that we need a utility to create temporary files and directories.
Instead of implementing that ourselves, we will use an existing crate.
Add the `tempfile` dependency to `Cargo.toml`:

```toml,customdiff
{{#include ../../gen/top_down/0_require_file/d_Cargo.toml.diff:4:}}
```

Note that this is dependency is added under `dev-dependencies`, indicating that this dependency is only available when running tests, benchmarks, and examples.
Therefore, users of our library will not depend on this library, which is good because temporary file creation is not necessary to users of our library.

Now, add the following tests to `src/fs.rs`:

```rust,
{{#include 0_require_file/e_fs_test.rs}}
```

The `tempfile` library takes care of deleting temporary files when they go out of scope (at the end of the test).
Unfortunately, we can't easily test when `metadata` and `open_if_file` should return an error, because we cannot disable read permissions on files via the Rust standard library.

Now we are done with our filesystem utility excursion.
Make the non-incremental context compatible by changing `src/context/non_incremental.rs`:

```rust,customdiff
{{#include ../../gen/top_down/0_require_file/f_non_incremental_context.rs.diff:4:}}
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
A dependency is consistent if after stamping, the new stamp equals the stored stamp.
If all dependencies of the task are consistent, we return the cached output of the task.
If not, we execute the task.

To implement this, we will need several components:
- `FileStamper` and `FileStamp` types for stamping files.
- `OutputStamper` and `OutputStamp` types for stamping task outputs.
- Extension to `Context` to support passing `FileStamper` and `OutputStamper` when requiring files and tasks.
- A `FileDependency` type that holds a `FileStamper` and `FileStamp` to check whether a file is consistent.
- A `TaskDependency` type that holds an `OutputStamper` and `OutputStamp` to check whether a task is consistent.
- A `Dependency` type that merges `FileDependency` and `TaskDependency` so we can check whether a dependency is consistent without having to know what kind of dependency it is.
- A `Store` type which holds the dependency graph with methods for interacting with the graph.
- A `TopDownContext` type that implements `Context` in an incremental way, using `Store`.

We will start with implementing stamps and dependencies, as that can be implemented as a stand-alone part.

### Stamp implementation

Add the `stamp` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/1_stamp/a_module.rs.diff:4:}}
```

Note that this module is declared `pub`, as users of the library should be able to construct stampers.

#### File stamps

Create the `src/stamp.rs` file and add:

```rust,
{{#include 1_stamp/b_file.rs}}
```

We're implementing `FileStamper` as an enum for simplicity.

A `FileStamper` has a single method `stamp` which takes something that can be dereferenced to a path, and produces a `FileStamp` or an error if creating the stamp failed.
For now, we implement only two kinds of file stampers: `Exists` and `Modified`.
The `Exists` stamper just returns a boolean indicating whether a file exists.
It can be used to create a file dependency where a task behaves differently based on whether a file exists or not.
The `Modified` stamper returns the last modification date if the file exists, or `None` if the file does not exist.

We derive `Eq` for stamps so that we can compare them.
Equal stamps indicate a consistent dependency, unequal indicates inconsistent.
We also derive `Eq` for stampers, because the stamper of a dependency could change, making the dependency inconsistent.

#### Task output stamps

We implement task output stampers in a similar way.
Add to `src/stamp.rs`:

```rust,
{{#include 1_stamp/c_output.rs}}
```

The `Inconsequential` stamper simply ignores the output and always returns the same stamp (thus is always equal).
It can be used to create a task dependency where we are interested in some side effect of a task, but don't care about its output.
The `Equals` stamper simply wraps the output of a task, so the stamp is equal when the output is equal.

Output stamps are generic over the task output type `O`.

```admonish info title="Trait bounds and derive macros" collapsible=true
Because `O` is used in the enum, the `derive` attributes on `OutputStamp` create bounds over `O`.
Thus, `OutputStamp` is only `Clone` when `O` is `Clone`, `OutputStamp` is only `Clone` when `O` is `Clone`, and so forth.
Because we declared `Task::Output` with bound `Clone + Eq + Debug`, we can be sure that `OutputStamp` is always `Clone`, `Eq`, and `Debug`.
```

```admonish info title="User definable stamps" collapsible=true
`FileStamper` and `OutputStamper` could also be a trait which would allow users of the library to implement their own stampers.
For simplicity, we do not explore that option in this tutorial.
If you feel adventurous, you could try to implement this after you've finished the tutorial.
Do note that this introduces a lot of extra generics and trait bounds everywhere, which can be a bit cumbersome.
```

#### Tests

Finally, we write some tests.
Add to `src/stamp.rs`:

```rust,
{{#include 1_stamp/d_test.rs}}
```

We test file stamps by creating a stamp, changing the file, creating a new stamp, and then compare the stamps.
We test task output stamps by just passing a different output value to the `stamp` function, and then compare the stamps.

Run `cargo test` to confirm the stamp implementation.

### Stamps in Context

We now have a module dedicated to stamps.
However, stampers are constructed by users of the library that author tasks, and they need to pass in these stampers when creating dependencies.
Therefore, we need to update the `Context` trait to allow passing in these stampers.

Change `Context` in `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/2_stamp_context/a_context.rs.diff:4:}}
```

We add the `require_task_with_stamper` and `require_file_with_stamper` methods which allow passing in a stamper.
We add default implementations for the `require_task` and `require_file` methods which pass in a default stamper.
The defaults are provided by `default_output_stamper` and `default_file_stamper` which can be overridden by the context implementation.

Update `NonIncrementalContext` in `src/context/non_incremental.rs` to implement the new methods:

```rust,customdiff
{{#include ../../gen/top_down/2_stamp_context/b_non_incremental_context.rs.diff:4:}}
```

We just ignore the stampers in `NonIncrementalContext`, as they are only needed for incrementality.

Run `cargo test` to confirm everything still works.

### Dependency implementation

Add the `dependency` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/top_down/3_dependency/a_module.rs.diff:4:}}
```

This module is not public, as users of the library should not construct dependencies.
They should only create stampers, which are passed to dependencies via the `Context`.

#### File dependencies

Create the `src/dependency.rs` file and add:

```rust,
{{#include 3_dependency/b_file.rs}}
```

A `FileDependency` stores the `path` the dependency is about, the `stamper` used to create a stamp for this dependency, and the `stamp` that was created at the time the file dependency was made.
The `FileDependency::new` function also returns the opened file if it exists, so that users of this function can read from the file without having to open it again.

A file dependency is inconsistent when the stored stamp is not equal to a stamp that we create at the time of checking, implemented in `FileDependency::is_inconsistent`.
For example, if we created a file dependency (with modified stamper) for a file that was modified yesterday, then modify the file, and then call `is_inconsistent` on the file dependency, it would return `Some(new_stamp)` indicating that the dependency is inconsistent.

We implement an `is_inconsistent` method here instead of an `is_consistent` method, so that we can return the changed stamp when the dependency is inconsistent, which we will use for debug logging purposes later.

Creating and checking a file dependency can fail due to file operations failing (for example, cannot access the file), so we propagate those errors.

#### Task dependencies

Task dependencies are implemented in a similar way.
Add to `src/dependency.rs`:

```rust,
{{#include 3_dependency/c_task.rs}}
```

A `TaskDependency` stores the `task` the dependency is about, along with its `stamper` and `stamp` that is created when the dependency is created.
Task dependencies are generic over the type of tasks `T`, and their type of outputs `O`.

```admonish info title="Trait bounds on structs" collapsible=true
We chose not to put a `Task` trait bound on `TaskDependency`, and instead put the bound on the impl.
There are several up and downsides to that should be considered when making such a decision.

The main upside for putting the `Task` bound on the `TaskDependency` struct, is that we can leave out `O` and use `OutputStamp<T::Output>` as the type of the `stamp` field.
This cuts down a generic parameter, which reduces boilerplate.
The downside is that we need to then put the `Task` bound on every struct that uses `TaskDependency`, which increases boilerplate.

Furthermore, some `derive` macros may behave differently or fail to work with trait bounds on tasks.
For example, the derive macros from `serde` which we will use for serialization later do not seem to work well with trait bounds on structs (or I did not figure out how to make them work).
Therefore, it is better to not put `Task` bounds on structs in this library.
```

A task dependency is inconsistent if, after recursively checking it, its stamp has changed, implemented in `TaskDependency::is_inconsistent`.
Usually, this will be using the `Equals` task output stamper, so a task dependency is usually inconsistent when the output of the task changes.

Because we need to recursively check the task, `TaskDependency::is_inconsistent` requires a context to be passed in.

#### Dependency enum

Finally, we create a `Dependency` enum that abstracts over these two kinds of dependencies.
Add to `src/dependency.rs`:

```rust,
{{#include 3_dependency/d_dependency.rs}}
```

`Dependency` just merges the two kinds of dependencies and provides an `is_inconsistent` method that calls the corresponding method.
This will make it easier to write a dependency graph implementation later.

We return the changed stamp here as well for debug logging later.
We wrap the changed stamp in an `InconsistentDependency` enum, and map to the correct variant if there is an inconsistency.

Because `Dependency` can store a `TaskDependency`, we need to propagate the `T` and `O` generics.
Likewise, `InconsistentDependency` propagates the `O` generic for `OutputStamp`.

```admonish info title="User definable dependencies" collapsible=true
Like with stampers, `Dependency` could also be a trait to allow users of the library to define their own dependencies.
However, as we will see later, these dynamic dependencies also require validation, and I am unsure how such a `Dependency` trait should look.
Therefore, we don't have an appendix on how to implement this.
But, if you have an idea on how to this nicely (after you've completed this tutorial), please get in touch! 
```

#### Tests

As usual, we write some tests to confirm the behaviour.
Add tests to `src/dependency.rs`:

```rust,
{{#include 3_dependency/e_test.rs}}
```

We test a file dependency by asserting that `is_inconsistent` returns `Some` after changing the file.

Testing task dependencies requires a bit more work.
We create task `ReadStringFromFile` that reads a string from a file, and then returns that string as output.
We require the task to get its output (`"test1"`), and create a task dependency with it.
Then, we change the file and check consistency of the task dependency.
That recursively requires the task, the context will execute the task, and the task now returns (`"test2"`).
Since we use the `Equals` output stamper, and `"test1"` does not equal `"test2"`, the dependency is inconsistent and returns a stamp containing `"test2"`.

Note that we are both testing the specific dependencies (`FileDependency` and `TaskDependency`), and the general `Dependency`.

```admonish
Normally, a task such as `ReadStringFromFile` shound return a `Result<String, io::Error>`, but for testing purposes we are just using panics with `expect`.

In the file dependency case, using `Dependency` requires an explicit type annotation because there is no task to infer the type from.
We just use `Dependency<ReadStringFromFile, String>` as the type, and this is fine even though we don't use `ReadStringFromFile` in that test, because the `Dependency::RequireFile` variant does not use those types. 
```

Run `cargo test` to confirm everything still works.
You will get some warnings about unused things, but that is ok as we will use them in the next section.
