# Incremental Top-Down Context

In this chapter, we will implement an *incremental* build context.
An incremental context selectively executes tasks — only those that are affected by a change.
In other words, an incremental context executes the *minimum number of tasks* required to make all tasks up-to-date.

However, due to dynamic dependencies, this is not trivial.
We cannot first gather all tasks into a dependency tree and then topologically sort that, as dependencies are added and removed *while tasks are executing*.
To do incremental builds in the presence of dynamic dependencies, we need to check and execute affected tasks *one at a time*, updating the dependency graph, while tasks are executing.
To achieve this, we will employ a technique called *top-down incremental building*, where we start checking if a top (root) task needs to be executed, and recursively check whether dependent tasks should be executed until we reach the bottom (leaf) task(s), akin to a depth-first search.

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
{{#include ../../gen/2_top_down/0_require_file/a_context.rs.diff:4:}}
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
{{#include ../../gen/2_top_down/0_require_file/b_fs_module.rs.diff:4:}}
```

Create file `src/fs.rs` with:

```rust,
{{#include 0_require_file/c_fs.rs}}
```

The `metadata` function gets the filesystem metadata given a path, and `open_if_file` opens the file for given path.
The reason for these functions is that the standard library function `std::fs::metadata` treats non-existent files as an error, whereas we don't want to treat it as an error and just return `None`.
Furthermore, `open_if_file` works around an issue where opening a directory on Windows (and possibly other operating systems) is an error, where we want to treat it as `None` again.
The documentation comments explain the exact behaviour.

We will write some tests to confirm the behaviour, but for that we need utilities to create temporary files and directories.
Furthermore, we will be writing more unit tests, integration tests, and even benchmarks in this tutorial, so we will set up these utilities in such a way that they are reachable by all these use cases.
The only way to do that in Rust right now, is to create a separate crate and have the `pie` crate depend on it.

TODO: create dev_shared crate
TODO: add tempfile and utilities to it
TODO: then have pie depend on dev_shared in dev-dependencies
TODO: modify tests to use this

Instead of implementing that ourselves, we will use an existing crate.
Add the `tempfile` dependency to `Cargo.toml`:

```toml,customdiff
{{#include ../../gen/2_top_down/0_require_file/d_Cargo.toml.diff:4:}}
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
{{#include ../../gen/2_top_down/0_require_file/f_non_incremental_context.rs.diff:4:}}
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
{{#include ../../gen/2_top_down/1_stamp/a_module.rs.diff:4:}}
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
{{#include ../../gen/2_top_down/2_stamp_context/a_context.rs.diff:4:}}
```

We add the `require_task_with_stamper` and `require_file_with_stamper` methods which allow passing in a stamper.
We add default implementations for the `require_task` and `require_file` methods which pass in a default stamper.
The defaults are provided by `default_output_stamper` and `default_file_stamper` which can be overridden by the context implementation.

Update `NonIncrementalContext` in `src/context/non_incremental.rs` to implement the new methods:

```rust,customdiff
{{#include ../../gen/2_top_down/2_stamp_context/b_non_incremental_context.rs.diff:4:}}
```

We just ignore the stampers in `NonIncrementalContext`, as they are only needed for incrementality.

Run `cargo test` to confirm everything still works.

### Dependency implementation

Add the `dependency` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/3_dependency/a_module.rs.diff:4:}}
```

This module is not public, as users of the library should not construct dependencies.
They should only create stampers, which are passed to dependencies via the `Context`.

#### File dependencies

Create the `src/dependency.rs` file and add:

```rust,
{{#include 3_dependency/b_file.rs}}
```

A `FileDependency` stores the `path` the dependency is about, the `stamper` used to create a stamp for this dependency, and the `stamp` that was created at the time the file dependency was made.
The `FileDependency::new_with_file` function also returns the opened file if it exists, so that users of this function can read from the file without having to open it again.

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
There are several up and downsides that should be considered when making such a decision.

The main upside for putting the `Task` bound on the `TaskDependency` struct, is that we can leave out `O` and use `OutputStamp<T::Output>` as the type of the `stamp` field.
This cuts down a generic parameter, which reduces boilerplate.
The downside is that we need to then put the `Task` bound on every struct that uses `TaskDependency`, which increases boilerplate.

In this case, we chose not to put the trait bound on the struct to prevent that trait bound from bubbling up into other structs that use `TaskDependency`, as it would need to appear in almost every struct in the library.
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

### Store implementation

To do incremental building, we need to keep track of all files, tasks, their dependencies, and task outputs.
This will be the responsibility of the `Store` data structure.
The `TopDownContext` and future context implementations will use methods on `Store` to request and update this data.
In other words, `Store` encapsulates this data.

Basically, `Store` will be a dependency graph.
However, writing a dependency graph data structure is outside of the scope of this tutorial, so we will be using the `pie_graph` library which we prepared exactly for this use case.
The graph from this library is a directed acyclic graph (DAG), meaning that edges are directed and there may be no cycles in edges, as that would prohibit topological orderings.

```admonish info title="Graph library" collapsible=true
The `pie_graph` library is a modified version of the great [`incremental-topo`](https://github.com/declanvk/incremental-topo/) library which implements incremental topological ordering: it keeps the topological ordering up-to-date incrementally while nodes and edges are added and removed.
That is exactly what we need, as dynamic dependencies prevents us from calculating the topological ordering in one go, and calculating the topological ordering after every task execution is prohibitively expensive.
The implementation in the `incremental-topo` library is based on a [paper by D. J. Pearce and P. H. J. Kelly](http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf) that describes several dynamic topological sort algorithms for directed acyclic graphs.
```

Add the `pie_graph` dependency to `Cargo.toml`:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/a_Cargo.toml.diff:4:}}
```

#### Store basics

Add the `store` module to `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/b_module.rs.diff:4:}}
```

This module is not public, as users of the library should not interact with the store.

Create the `src/store.rs` file and add the following to get started:

```rust,
{{#include 4_store/c_basic.rs}}
```

The `Store` is generic over tasks `T` and their outputs `O`, like we have done before with `Dependency`.

The `DAG` type from `pie_graph` represents a DAG with nodes and edges, and data attached to those nodes and edges.
The nodes in our graph are either files or tasks, and the edges are dependencies.

The first generic argument to `DAG` is the type of data to attach to nodes, which is `NodeData<T, O>` in our case.
Because nodes can be files or tasks, `NodeData<T, O>` enumerates these, storing the path for files, and the task along with its output for tasks.
We store file paths as `PathBuf`, which is the owned version of `Path` (similar to `String` being the owned version of `str`).
The task output is stored as `Option<O>` because we can add a task to the graph without having executed it, so we don't have its output yet.

The second argument is the type of data to attach to edges, which is `Dependency<T, O>`, using the `Dependency` enum we defined earlier.

We implement `Default` for the store to initialize it.

```admonish info title="Deriving default" collapsible=true
We cannot derive this `Default` implementation even though it seems we should be able to, because the derived implementation will require `T` and `O` to be `Default`, and this is not always the case.
This is because the `Default` derive macro is conservative and adds a `: Default` bound to *every* generic argument in the `Default` trait implementation, and there is no way to disable this behaviour.
Therefore, we implement `Default` ourselves.

There are several crates that have more configurable derive macros for these things, but adding an extra dependency to generate a few lines of code is not worth the extra compilation time, so we just implement it manually here.
```

#### Graph nodes

A node in `DAG` is represented by a `Node`, which is a transparent identifier (sometimes called a [handle](https://en.wikipedia.org/wiki/Handle_(computing))) that points to the node and its data.
We can create nodes in the graph, and then query attached data (`NodeData`) given a node.
So `DAG` allows us to go from `Node` to a `PathBuf` and task `T` through attached `NodeData`.

However, we want each unique file and task to be represented by a single unique node in the graph.
We need this for incrementality so that if the build system encounters the same task twice, we can find the corresponding task node in the graph the second time, check if it is consistent, and return its output if it is.

To ensure unique nodes, we need to maintain the reverse mapping from `PathBuf` and `T` to `Node` ourselves, which we will do with `HashMap`s.
This is also the reason for the `Eq` and `Hash` trait bounds on the `Task` trait, so we can use them as keys in `HashMap`s.

Change `src/store.rs` to add hash maps to map between these things:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/d_mapping_diff.rs.diff:4:}}
```

Furthermore, we also create the `FileNode` and `TaskNode` [newtypes](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) here so that we can prevent a file node to be accidentally used as a task node, and vice versa.
The `Borrow` implementations will make subsequent code a bit more concise by automatically converting `&FileNode` and `&TaskNode`s to `&Node`s.

Now we will add methods create nodes and to query their attached data.
Add the following code to `src/store.rs`:

```rust,
{{#include 4_store/e_mapping.rs}}
```

The `get_or_create_file_node` method creates file nodes.
When we want to go from a file path (using `impl AsRef<Path>`) to a `FileNode`, either we have already added this file path to the graph and want to get the `FileNode` for it, or we have not yet added it to the graph yet and should add it.
The former is handled by the if branch in `get_or_create_file_node`, where we just retrieve the `FileNode` from the `file_to_node` hash map.
The latter is handled by the else branch where we add the node to the graph with `graph.add_node` which attaches the `NodeData::File` data to the node, and then returns a `FileNode` which we insert into the `file_to_node` map.

The `get_file_path` method does the inverse.
We get the attached data given a node, and extract the file path from it.

Note that we are using `panic!` here to indicate that invalid usage of this method is an *unrecoverable programming error* that should not occur.
Returning an `Option<&PathBuf>` makes no sense here, as the caller of this method has no way to recover from this.
Because this is not an end-user-facing API (`store` module is private), we control all the calls to this method, and thus we are responsible for using these methods in a valid way. 
Therefore, when we call these methods, we should document why it is valid (if this is not immediately obvious), and we need to test whether we really use it in a valid way.

We're also documenting the panics in a `# Panics` section in the documentation comment, as is common practice in Rust.

```admonish info title="Triggering these panics" collapsible=true
Because only `Store` can create `FileNode`s and `TaskNode`s, and all methods only take these values as inputs, these panics will not happen under normal usage.
The only way to trigger these panics (in safe Rust) would be to create two stores, and use the nodes from one store in another.
However, since this is a private module, we just need to make sure that we don't do that.

There are some tricks to prevent even this kind of invalid usage.
For example, the [generativity](https://docs.rs/generativity/latest/generativity/) crate generates unique identifiers based on lifetimes.
However, that is a bit overkill, especially for an internal API, so we won't be using that.
```

We implement similar methods for task nodes in `get_or_create_task_node` and `get_task`.

#### Task outputs

When we do not need to execute a task because it is consistent, we still need to return its output.
Therefore, we store the task output in `NodeData::Task` and add methods to query and manipulate task outputs.
Add the following code to `src/store.rs`:

```rust,
{{#include 4_store/f_output.rs}}
```

The `task_has_output`, `get_task_output`, and `set_task_output` methods manipulate task outputs in `NodeData::Task`.

Again, we are using panics here to indicate unrecoverable programming errors.

#### Dependencies

Now we need methods to query and manipulate dependencies.
The edges in the graph are dependencies between tasks and files.
Tasks can depend on other tasks and files, but there are no dependencies between files.
An edge does not have its own dedicated representation, and is simply represented by two nodes: the source node and the destination node of the edge.

Add the following code to `src/store.rs`:

```rust,
{{#include 4_store/g_dependency.rs}}
```

The `get_dependencies_of_task` method gets the dependencies (edge data of outgoing edges) of a task.
We're using `debug_assert!` here to trigger a panic indicating an unrecoverable programming error only in development mode, because this check is too expensive to run in release (optimized) mode.

The `add_file_require_dependency` method adds a file dependency.
Adding an edge to the graph can result in cycles, which are not allowed in a directed *acyclic* graph (DAG).
Therefore, `graph.add_edge` can return an `Err` indicating that there is a cycle.
In case of files, this cannot happen because files do not have outgoing dependencies, and the API enforces this by never taking a `FileNode` as a source (`src`) of an edge.

Tasks can depend on other tasks, so they can create cycles.
In `add_task_require_dependency`, we propagate the cycle detected error (by returning `Err(())`) to the caller because the caller has more information to create an error message for the user that made a cyclic task dependency.

[//]: # (We want to catch those cycles not just because they are not allowed in a DAG, but because cycles between tasks could result in cyclic task execution, causing builds to infinitely execute.)

[//]: # ()
[//]: # (When a task requires another task, we need to add an edge to the dependency graph to check if this edge creates a cycle, but also to have this edge in the dependency graph for future cycle detection.)

[//]: # (However, at the moment we require the task, we do not yet have an output for the task, as we only get an output after we've executed the task.)

[//]: # (Therefore, we first *reserve* the task dependency, and then update it with an output.)

[//]: # (This manifests itself as the attached edge data being `Option<Dependency<T, O>>`, where `None` indicates that the dependency has been reserved.)

[//]: # ()
[//]: # (The `reserve_task_require_dependency` and `update_reserved_task_require_dependency` methods implement this behavior.)

[//]: # (We propagate cycle errors so that the caller can report an error message.)

#### Resetting tasks

Finally, when we determine that a task is inconsistent and needs to be executed, we first need to remove its output and remove its outgoing dependencies, as those will interfere with incrementality when not removed.
Add the `reset_task` method that does this to `src/store.rs`:

```rust,
{{#include 4_store/h_reset.rs}}
```

Now we've implemented everything we need for implementing the top-down context, but first we will write some tests.

#### Tests

TODO

### Top-down context implementation

#### Top-down context basics

Add the `top_down` module to `src/context/mod.rs`:

```rust,customdiff

```

Create the `src/context/top_down.rs` file and add the following to get started:

```rust,
{{#include 4_top_down/initial_context.rs}}
```

### Top-down context implementation
