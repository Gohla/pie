# Non-Incremental Context

We set up the `Task` and `Context` API in such a way that we can implement incrementality.
However, incrementality is *hard*, so let's start with an extremely simple non-incremental `Context` implementation to get a feeling for the API.

## Context module

Since we will be implementing three different contexts in this tutorial, we will separate them in different modules.
Create the `context` module by adding a module to `pie/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/1_programmability/2_non_incremental/a_context_module.rs.diff
```

This is a diff over `pie/src/lib.rs` where lines with a green background are additions, lines with a red background are removals, lines without a special background are context on where to add/remove lines, and lines starting with `@@` denote changed lines (in unified diff style). This is similar to diffs on source code hubs like GitHub.

Create the `pie/src/context` directory, and in it, create the `pie/src/context/mod.rs` file with the following contents:

```rust,
{{#include b_non_incremental_module.rs}}
```

Both modules are public so that users of our library can access context implementations.

Create the `pie/src/context/non_incremental.rs` file, it will be empty for now.
Your project structure should now look like:

```
{{#include ../../gen/1_programmability/2_non_incremental/b_dir.txt}}
```

Confirm your module structure is correct by building with `cargo build`.

```admonish tip title="Rust Help: Modules, Visibility" collapsible=true
Modules are typically [separated into different files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html).
Modules are declared with `mod context`. 
Then, the contents of a module are defined either by creating a sibling file with the same name: `context.rs`, or by creating a sibling directory with the same name, with a `mod.rs` file in it: `context/mod.rs`.
Use the latter if you intend to nest modules, otherwise use the former.

Like traits, modules also have [visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html).
```

## Implementation

Implement the non-incremental context in `pie/src/context/non_incremental.rs` by adding:

```rust,
{{#include c_non_incremental_context.rs}}
```

This `NonIncrementalContext` is extremely simple: in `require_task` we unconditionally execute the task, and pass `self` along so the task we're calling can require additional tasks.
Let's write some tests to see if this does what we expect.

```admonish tip title="Rust Help: Crates (Libraries), Structs, Trait Implementations, Last Expression" collapsible=true
In Rust, libraries are called [crates](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html).
We import the `Context` and `Task` traits from the root of your crate (i.e., the `src/lib.rs` file) using `crate::` as a prefix.

[Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html) are concrete types that can contain data through fields and implement traits, similar to classes in class-oriented languages.
Since we don't need any data in `NonIncrementalContext`, we define it as a [unit-like struct](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#unit-like-structs-without-any-fields).

[Traits are implemented for a type](https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type) with `impl Context for NonIncrementalContext { ... }`, where we then have to implement all methods and associated types of the trait.

The `Context` trait is generic over `Task`, so in the `impl` block we introduce a type parameter `T` with `impl<T>`, and use [trait bounds](https://doc.rust-lang.org/book/ch10-02-traits.html#using-trait-bounds-to-conditionally-implement-methods) as `impl<T: Task>` to declare that `T` must implement `Task`.

The last expression of a function – in this case `task.execute(self)` in `require_task` which is an expression because it does not end with `;` – is used as the return value.
We could also write that as `return task.execute(self);`, but that is more verbose.
```

## Simple Test

Add the following test to `pie/src/context/non_incremental.rs`:

```rust,
{{#include d_test.rs}}
```

In this test, we create a struct `ReturnHelloWorld` which is the "hello world" of the build system.
We implement `Task` for it, set its `Output` associated type to be `String`, and implement the `execute` method to just return `"Hello World!"`.
We derive the `Clone`, `Eq`, `Hash`, and `Debug` traits for `ReturnHelloWorld` as they are required for all `Task`
implementations.

We require the task with our context by creating a `NonIncrementalContext`, calling its `require_task` method, passing
in a reference to the task.
It returns the output of the task, which we test with `assert_eq!`.

Run the test by running `cargo test`.
The output should look something like:

```shell,
{{#include ../../gen/1_programmability/2_non_incremental/d_cargo.txt}}
```

Which indicates that the test indeed succeeds!
You can experiment by returning a different string from `ReturnHelloWorld::execute` to see what a failed test looks like.

```admonish tip title="Rust Help: Unit Testing, Nested Items, Unused Parameters, Assertion Macros" collapsible=true
[Unit tests](https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-module-and-cfgtest) for a module 
are typically defined by creating a nested module named `test` with the `#[cfg(test)]` attribute applied to it. In that
`test` module, you apply `#[test]` to testing functions, which then get executed when you run `cargo test`.

The `#[cfg(...)]` attribute provides [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html) for the item it is applied to. In this case, `#[cfg(test)]` ensures that the module is only compiled when we run `cargo test`.

We import all definitions from the parent module (i.e., the `non_incremental` module) into the `test` module with `use super::*;`.

In Rust, [items](https://doc.rust-lang.org/reference/items.html) — that is, functions, structs, implementations, etc. — 
can be nested inside functions. We use that in `test_require_task_direct` to scope `ReturnHelloWorld` and its implementation
to the test function, so it can't clash with other test functions.

In `execute`, we use `_context` as the parameter name for the context, as the parameter is unused.
Unused parameters give a warning in Rust, unless it is prefixed by a `_`.

[assert_eq!](https://doc.rust-lang.org/std/macro.assert_eq.html) is a [macro](https://doc.rust-lang.org/book/ch19-06-macros.html) that checks if its two expressions are equal. 
If not, it [panics](https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html).
This macro is typically [used in tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) for assertions, as a panic marks a test as failed.
```

## Test with Multiple Tasks

Our first test only tests a single task that does not use the context, so let's write a test with two tasks where one requires the other to increase our test coverage.
Add the following test:

```diff2html fromfile linebyline
../../gen/1_programmability/2_non_incremental/e_test_problematic.rs.diff
```

We use the same `ReturnHelloWorld` task as before, but now also have a `ToLowerCase` task which requires `ReturnHelloWorld` and then turn its string lowercase.
However, due to the way we've set up the types between `Task` and `Context`, we will run into a problem.
Running `cargo test`, you should get these errors:

```shell,
{{#include ../../gen/1_programmability/2_non_incremental/e_cargo.txt}}
```

The problem is that `execute` of `ToLowerCase` takes a `Context<Self>`, so in `impl Task for ToLowerCase` it takes a `Context<ToLowerCase>`, while we're trying to require `&ReturnHelloWorld` through the context.
This doesn't work as `Context<ToLowerCase>::require_task` only takes a `&ToLowerCase` as input.

We could change `execute` of `ToLowerCase` to take `Context<ReturnHelloWorld>`:

```diff2html fromfile
../../gen/1_programmability/2_non_incremental/f_test_incompatible.rs.diff
```

But that is not allowed:

```shell,
{{#include ../../gen/1_programmability/2_non_incremental/f_cargo.txt}}
```

This is because the `Task` trait defines `execute` to take a `Context<Self>`, thus every implementation of `Task` must adhere to this, so we can't solve it this way.

Effectively, due to the way we defined `Task` and `Context`, we can only use *a single task implementation*.
This is to simplify the implementation in this tutorial, as supporting multiple task types complicates matters a lot.

```admonish question title="Why only a Single Task Type?" collapsible=true
Currently, our context is parameterized by the type of tasks: `Context<T>`.
Again, this is for simplicity.

An incremental context wants to build a *single dependency graph* and cache task outputs, so that we can figure out from the graph whether a task is affected by a change, and just return its output if it is not affected.
Therefore, a context implementation will maintain a `Store<T>`.

Consider the case with two different task types
A `Context<ReturnHelloWorld>` and `Context<ToLowerCase>` would then have a `Store<ReturnHelloWorld>` and `Store<ToLowerCase>` respectively.
These two stores would then maintain two different dependency graphs, one where the nodes in the graph are `ReturnHelloWorld` and one where the nodes are `ToLowerCase`.
But that won't work, as we need a single dependency graph over all tasks to figure out what is affected.
Therefore, we are restricted to a single task type in this tutorial.

To solve this, we would need to remove the `T` generic parameter from `Context`, and instead use [trait objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html).
However, this introduces a whole slew of problems because many traits that we use are not inherently trait-object safe. 
`Clone` is not object safe because it requires `Sized`. 
`Eq` is not object safe because it uses `Self`. 
Serializing trait objects is problematic.
There are workarounds for all these things, but it is not pretty and very complicated.

The actual PIE library supports arbitrary task types through trait objects.
We very carefully control where generic types are introduced, and which traits need to be object-safe.
Check out the PIE library if you want to know more! 
```

For now, we will solve this by just using a single task type which is an enumeration of the different possible tasks.
First remove the problematic test:

```diff2html fromfile linebyline
../../gen/1_programmability/2_non_incremental/g_remove_test.rs.diff
```

Then add the following test:

```diff2html fromfile linebyline
../../gen/1_programmability/2_non_incremental/h_test_correct.rs.diff
```

Here, we instead define a single task `Test` which is an `enum` with two variants.
In its `Task` implementation, we match ourselves and return `"Hello World!"` when the variant is `ReturnHelloWorld`.
When the variant is `ReturnHelloWorld`, we require `&Self::ReturnHelloWorld` through the context, which is now valid because it is an instance of `Test`, and turn its string lowercase and return that.
This now works due to only having a single task type.
Run the test with `cargo test` to confirm it is working.

```admonish tip title="Rust Help: Enum" collapsible=true
[Enums](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html) define a type by a set of variants, similar to enums in other languages, sometimes called tagged unions in other languages.
The `match` expression matches the variant and dispatches based on that, similar to switch statements in other languages.
```

We have defined the API for the build system and implemented a non-incremental version of it.
We're now ready to start implementing an incremental context in the next chapter.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/1_programmability/2_non_incremental/source.zip).
```
