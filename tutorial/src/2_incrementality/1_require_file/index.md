# Requiring Files

Since build systems frequently interact with files, and changes to files can affect tasks, we need to keep track of file dependencies.
Therefore, we will extend the `Context` API with methods to *require files*, enabling tasks to specify dynamic dependencies to files.

Add a method to the `Context` trait in `pie/src/lib.rs`:

```diff2html fromfile
../../gen/2_incrementality/1_require_file/a_context.rs.diff
```

`require_file` is similar to requiring a task, but instead takes a `path` to a file or directory on the filesystem as input.
We use `AsRef<Path>` as the type for the path, so that we can pass anything in that can dereference to a path.
For example, `str` has an `AsRef<Path>` implementation, so we can just use `"test.txt"` as a path.

As an output, we return `Result<Option<File>, io::Error>`, with `File` being a handle to an open file.
The reason for this complicated type is:

* An incremental context will want to read the metadata (such as the last modified date) of the file, or create a hash over the file, to be able to detect changes. Because getting metadata or reading the file can fail, and we want to propagate this error, we return a `Result` with `io::Error` as the error type.
* Tasks can create a dependency to a file that does not exist, and the existence of that file affects the task. For example, a task that prints true or false based on if a file exists. If the file does not exist (or it is a directory), we cannot open it, so we cannot return a `File`, hence we use `Option<File>` to return `None`.
* Otherwise, we return `Ok(Some(file))` so that the task can read the opened file.

```admonish tip title="Rust Help: Error Handling with Result, Optional, AsRef Conversion" collapsible=true
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

Add the `fs` module to `pie/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/1_require_file/b_fs_module.rs.diff
```

Create file `pie/src/fs.rs` with:

```rust,
{{#include c_fs.rs}}
```

The `metadata` function gets the filesystem metadata given a path, and `open_if_file` opens the file for given path.
The reason for these functions is that the standard library function `std::fs::metadata` treats non-existent files as an error, whereas we don't want to treat it as an error and just return `None`.
Furthermore, `open_if_file` works around an issue where opening a directory on Windows (and possibly other operating systems) is an error, where we want to treat it as `None` again.
The documentation comments explain the exact behaviour.

```admonish tip title="Rust Help: Error Propagation, Documentation Comments" collapsible=true
The `?` operator makes it easy to [propgate errors](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator).
Because errors are just values in Rust, to propgate an error, you'd normally have to match each result and manually propagate the error.
The `r?` operator applied to a `Result` `r` does this for you, it basically desugars to something like `match r { Err(e) => return Err(e), _ => {} }`.

Comments with three forward slashes `///` are [documentation comments](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments) that document the function/struct/enum/trait/etc. they are applied to.
```

We will write some tests to confirm the behaviour, but for that we need utilities to create temporary files and directories.
Furthermore, we will be writing more unit tests, integration tests, and even benchmarks in this tutorial, so we will set up these utilities in such a way that they are reachable by all these use cases.
The only way to do that in Rust right now, is to create a separate crate and have the `pie` crate depend on it.

And yes, we went from adding file dependencies, to creating file system utilities, to testing those file system utilities, to creating testing utilities, and now to making a crate for those testing utilities.
Sorry about that 😅, we will start unwinding this stack soon!

Next to the `pie` directory, create a directory named `dev_shared`.
Create the `dev_shared/Cargo.toml` file with the following contents:

```toml,
{{#include d_dev_shared_Cargo.toml}}
```

We've added the `tempfile` dependency here already, which is a crate that creates and automatically cleans up temporary files and directories.

```admonish tip title="Rust Help: Dependencies" collapsible=true
We use other libraries (crates) by [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).
Because basically every Rust library adheres to [semantic versioning](https://semver.org/), we can use `"3"` as a version requirement which indicates that we will use the most up-to-date `3.x.x` version.
```

Create the main library file `dev_shared/src/lib.rs`, with functions for creating temporary files and directories:

```rust,
{{#include e_dev_shared_lib.rs}}
```

Your directory structure should now look like this:

```
{{#include ../../gen/2_incrementality/1_require_file/e_dir.txt:2:}}
```

To access these utility functions in the `pie` crate, add a dependency to `dev_shared` in `pie/Cargo.toml` along with another create that will help testing:

```diff2html fromfile linebyline
../../gen/2_incrementality/1_require_file/f_Cargo.toml.diff
```

We've also added the [assert_matches](https://crates.io/crates/assert_matches) crate, which is a handy library for asserting that a value matches a pattern.
Note that these dependencies are added under `dev-dependencies`, indicating that these dependencies are only available when running tests, benchmarks, and examples.
Therefore, users of our library will not depend on these crates, which is good, because temporary file management and assertions are not necessary to users of the library.

Back to testing our filesystem utilities.
Add the following tests to `pie/src/fs.rs`:

```rust,
{{#include g_fs_test.rs}}
```

We test whether the functions conform to the specified behaviour.
Unfortunately, we can't easily test when `metadata` and `open_if_file` should return an error, because we cannot disable read permissions on files via the Rust standard library.

We use our `create_temp_file` and `create_temp_dir` utility functions to create temporary files and directories.
The `tempfile` library takes care of deleting temporary files when they go out of scope (at the end of the test).

We use `assert_matches!` to assert that `metadata` is `Some(metadata)`, binding `metadata` in the ` => { ... }` block in which we assert that the metadata describes a file.
We will use this macro more in future integration tests.

```admonish tip title="Rust Help: Result in Tests" collapsible=true
Tests can [return `Result`](https://doc.rust-lang.org/book/ch11-01-writing-tests.html#using-resultt-e-in-tests).
When a test returns an `Err`, the test fails.
This allows us to write more concise tests using error propagation.
```

Now we are done unwinding our stack and have filesystem and testing utilities.
Make the non-incremental context compatible by changing `pie/src/context/non_incremental.rs`:

```diff2html fromfile
../../gen/2_incrementality/1_require_file/h_non_incremental_context.rs.diff
```

Since the non-incremental context does not track anything, we only try to open the file and return it, matching the contract in the documentation comment of the `Context::require_file` trait method.

Confirm everything works with `cargo test`.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/1_require_file/source.zip).
```
