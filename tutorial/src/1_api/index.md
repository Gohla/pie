# Programmable Build System API

In this first chapter, we will program the core API of the programmatic incremental build system, and implement an extremely simple non-incremental version of the build system to get started.

## Task and Context

The unit of computation in a programmatic build system is a *task*.
A task is kind of like a closure: a value that can be executed to produce their output.
However, in an *incremental* programmatic build system, we also need to keep track of *dynamic dependencies* that are made while tasks are executing.
Therefore, tasks are executed under a *build context* which enable them to create these dependencies.
Tasks *require* other tasks through the context, creating a dynamic dependency and returning their up-to-date output.

On the other hand, an incremental build context wants to *selectively execute tasks* — only those that are affected by a change.
To that end, a build context will selectively execute tasks, tasks require other tasks through the build context, the build context selectively executes those, and so forth.
Thus, tasks and build contexts are mutually recursive.

In this tutorial, we will be using the words *context*, *build context*, and *build system* interchangeably, typically using just *context* as it is concise.

Let's make tasks and contexts more concrete by defining them in code.

### API Implementation

Since we want users of the build system to implement their own tasks, we will define `Task` as a trait.
Likewise, we will also be implementing multiple contexts in this tutorial, so we will also define `Context` as a trait.
Add the following code to your `pie/src/lib.rs` file:

```rust,
{{#include 0_api/a_api.rs}}
```

```admonish
If this seems overwhelming to you, don't worry. We will go through the API and explain things. But more importantly, the API should become more clear once we implement it in the next section and subsequent chapters.
Furthermore, if you're new to Rust and/or need help understanding certain concepts, I will try to explain them in Rust Help blocks. They are collapsed by default to reduce distraction, clicking the header opens them. See the first Rust Help block at the end of this section.
```

The `Task` trait has several supertraits that we will need later in the tutorial to implement incrementality:

* `Eq` and `Hash`: to check whether a task is equal to another one, and to create a hash of it, so we can use
  a `HashMap` to get the output of a task if it is up-to-date.
* `Clone`: to create a clone of the task so that we can store it in the `HashMap` without having ownership of it.
* `Debug`: to format the task for debugging purposes.

A `Task` has a single method `execute`, which takes a reference to itself (`&self`), and a mutable reference to a context (`context: &mut C`), and produces a value of type `Self::Output`.
Because `Context` is a trait, we use generics (`<C: Context<Self>>`) to have `execute` work for any `Context` implementation (ignoring the `Self` part for now).
The `execute` method takes self by reference such that a task can access its data, but not mutate it, as that could throw off incrementality by changing the hash/equality of the task.
Finally, the type of output of a task is defined by the `Output` associated type, and this type must implement `Clone`, `Eq`, and `Debug` for the same reason as `Task`.

The `Context` trait is generic over `Task`, allowing it to work with any task implementation.
It has a single method `require_task` for creating a dependency to a task and returning its up-to-date result.
It takes a mutable reference to itself, enabling dependency tracking and caching, which require mutation.
Because of this, the context reference passed to `Task::execute` is also mutable.

This `Task` and `Context` API mirrors the mutually recursive definition of task and context we discussed earlier, and forms the basis for the entire build system.

Build the project by running `cargo build`.
The output should look something like:

```shell,
{{#include ../../gen/1_api/0_api/a_cargo.txt}}
```

```admonish info title="Rust Help" collapsible=true
[The Rust Programming Language](https://doc.rust-lang.org/book/ch00-00-introduction.html) is an introductory book about Rust. I will try to provide links to the book where possible.

Rust has a [module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) for project organization. The `lib.rs` file is the "main file" of a library. Later on, we will be creating more modules in different files.

Things are imported into the current scope with [`use`](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html) statements. We import the `Debug` and `Hash` traits from the standard library with two `use` statements. Use statements use [paths](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html) to refer to nested things. We use `::` for nesting, similar to namespaces in C++.

Rust models the concept of [ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html) to enable memory safety without a garbage collector.
The `execute` method accepts a *reference* to the current type, indicated with `&`: `&self`. This reference is *immutable*, meaning that we can read data from it, but not mutate it. In Rust, things are immutable by default.
On the other hand, `execute` accepts a *mutable reference* to the context, indicated with `&mut`: `context: &mut C`, which does allow mutation.

[Traits](https://doc.rust-lang.org/book/ch10-02-traits.html) are the main mechanism for open extensibility in Rust. They are comparable to interfaces in class-oriented languages. We will implement a context and tasks in the next section.

[Supertraits](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-supertraits-to-require-one-traits-functionality-within-another-trait) are a kind of inheritance. The `: Clone + Eq + Hash + Debug` part of the `Task` trait means that every `Task` implementation must also implement the `Clone`, `Eq`, `Hash`, and `Debug` traits. These traits are part of the standard library:
* [Clone](https://doc.rust-lang.org/std/clone/trait.Clone.html) for duplicating values.
* [Eq](https://doc.rust-lang.org/std/cmp/trait.Eq.html) for equality comparisons, along with [PartialEq](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html).
* [Hash](https://doc.rust-lang.org/std/hash/trait.Hash.html) for turning a value into a hash.
* [Debug](https://doc.rust-lang.org/std/fmt/trait.Debug.html) for formatting values in a programmer-facing debugging context.

`Clone` and `Eq` are so common that they are part of the [Rust Prelude](https://doc.rust-lang.org/std/prelude/index.html), so we don't have to import those with `use` statements.

[Methods](https://doc.rust-lang.org/book/ch05-03-method-syntax.html) are functions that take a form of `self` as the first argument. This enables convenient object-like calling syntax: `context.require_task(&task);`.

[Associated types](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#specifying-placeholder-types-in-trait-definitions-with-associated-types) are a kind of placeholder type in a trait such that methods of traits can use that type. In `Task` this allows us to talk about the `Output` type of a task. In `Context` this allows us to refer to both the `Task` type `T` and its output type `T::Output`. The `::` syntax here is used to access associated types of traits.

The `Self` type in a trait is a built-in associated type that is a placeholder for the type that is implementing the trait.

The `Task` trait is defined with `pub` (public) [visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html), such that users of the library can implement it. Because `Task` uses `Context` in its public API, `Context` must also be public, even though we don't intend for users to implement their own `Context`. 
```

## Non-Incremental Context

We set up the `Task` and `Context` API in such a way that we can implement incrementality.
However, incrementality is *hard*, so let's start with an extremely simple non-incremental `Context` implementation to get a feeling for the API.

### Context module

Since we will be implementing three different contexts in this tutorial, we will separate them in different modules.
Create the `context` module by adding a module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/1_api/1_non_incremental/a_context_module.rs.diff:4:}}
```

This is a diff over `pie/src/lib.rs` where lines with a green background are additions, lines with a red background are removals, and lines with a grey background are context on where to add/remove lines, similar to diffs on source code hubs like GitHub.

Create the `pie/src/context` directory, and in it, create the `pie/src/context/mod.rs` file with the following contents:

```rust,
{{#include 1_non_incremental/b_non_incremental_module.rs}}
```

Then, create the `pie/src/context/non_incremental.rs` file, it will be empty for now.
Your project structure should now look like:

```
{{#include ../../gen/1_api/1_non_incremental/b_dir.txt}}
```

Confirm your module structure is correct by building with `cargo build`.

```admonish info title="Rust Help" collapsible=true
Modules are typically [separated into different files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html).
Modules are declared with `mod context`. 
Then, the contents of a module are defined either by creating a sibling file with the same name: `context.rs`, or by creating a sibling directory with the same name, with a `mod.rs` file in it: `context/mod.rs`.
Use the latter if you intend to nest modules, otherwise use the former.

Like traits, modules also have [visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html).
```

### Implementation

Implement the non-incremental context in `pie/src/context/non_incremental.rs` by adding:

```rust,
{{#include 1_non_incremental/c_non_incremental_context.rs}}
```

This `NonIncrementalContext` is extremely simple: in `require_task` we unconditionally execute the task, and pass `self` along so the task we're calling can require additional tasks.
Let's write some tests to see if this does what we expect.

```admonish info title="Rust Help" collapsible=true
In Rust, libraries are called [crates](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html).
We import the `Context` and `Task` traits from the root of your crate (i.e., the `src/lib.rs` file) using `crate::` as a prefix.

[Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html) are concrete types that can contain data through fields and implement traits, similar to classes in class-oriented languages.
Since we don't need any data in `NonIncrementalContext`, we define it as a [unit-like struct](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#unit-like-structs-without-any-fields).

[Traits are implemented for a type](https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type) with `impl Context for NonIncrementalContext { ... }`, where we then have to implement all methods and associated types of the trait.

The `Context` trait is generic over `Task`, so in the `impl` block we introduce a type parameter `T` with `impl<T>`, and use [trait bounds](https://doc.rust-lang.org/book/ch10-02-traits.html#using-trait-bounds-to-conditionally-implement-methods) as `impl<T: Task>` to declare that `T` must implement `Task`.

The last expression of a function – in this case `task.execute(self)` in `require_task` which is an expression because it does not end with `;` – is used as the return value.
We could also write that as `return task.execute(self);`, but that is more verbose.
```

### Simple Test

Add the following test to `pie/src/context/non_incremental.rs`:

```rust,
{{#include 1_non_incremental/d_test.rs}}
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
{{#include ../../gen/1_api/1_non_incremental/d_cargo.txt}}
```

Which indicates that the test indeed succeeds!
You can experiment by returning a different string from `ReturnHelloWorld::execute` to see what a failed test looks like.

```admonish info title="Rust Help" collapsible=true
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

### Test with Multiple Tasks

Our first test only tests a single task that does not use the context, so let's write a test with two tasks where one requires the other to increase our test coverage.
Add the following test:

```rust,customdiff
{{#include ../../gen/1_api/1_non_incremental/e_test_problematic.rs.diff:4:}}
```

We use the same `ReturnHelloWorld` task as before, but now also have a `ToLowerCase` task which requires `ReturnHelloWorld` and then turn its string lowercase.
However, due to the way we've set up the types between `Task` and `Context`, we will run into a problem.
Running `cargo test`, you should get these errors:

```shell,
{{#include ../../gen/1_api/1_non_incremental/e_cargo.txt}}
```

The problem is that `execute` of `ToLowerCase` takes a `Context<Self>`, so in `impl Task for ToLowerCase` it takes a `Context<ToLowerCase>`, while we're trying to require `&ReturnHelloWorld` through the context.
This doesn't work as `Context<ToLowerCase>::require_task` only takes a `&ToLowerCase` as input.

We could change `execute` of `ToLowerCase` to take `Context<ReturnHelloWorld>`:

```rust,customdiff
{{#include ../../gen/1_api/1_non_incremental/f_test_incompatible.rs.diff:4:}}
```

But that is not allowed:

```shell,
{{#include ../../gen/1_api/1_non_incremental/f_cargo.txt}}
```

This is because the `Task` trait defines `execute` to take a `Context<Self>`, thus every implementation of `Task` must adhere to this, so we can't solve it this way.

Effectively, due to the way we defined `Task` and `Context`, we can only use *a single task implementation*.
However, there is a good reason for this which will become more apparent once we implement incrementality.

```admonish info title="Why only a single Task type?" collapsible=true
The gist of it is that an incremental context wants to build a dependency graph and cache task outputs, so that we can figure out from the dependency graph whether a task is affected by a change, and just return its output if it is not affected.
For that, a context implementation will have a `Store<T>`.

A `Context<ReturnHelloWorld>` and `Context<ToLowerCase>` would have to be implemented by different types, which would have different `Store`s, which would then have completely separate dependency graphs.
That won't work, as we need a single (global) dependency graph over all tasks to figure out what is affected.
```

[//]: # (Using trait objects, but this introduces a whole slew of problems because many traits that we use are not trait-object safe. `Clone` is not compatible because it requires `Sized`. `Eq` is not compatible because it uses `Self`. Serializing trait-objects is problematic. There are workarounds for all these things, but it is not pretty and very complicated.)

There is a much more complicated way to actually solve this problem, but it introduces too much complexity into the tutorial, so we will be going with a much simpler solution.
In chapter *TODO*, we will describe this solution which does support multiple task types.

For now, we will solve this by just using a single task type which is an enumeration of the different possible tasks.
Replace the test with the following:

```rust,customdiff
{{#include 1_non_incremental/g_test_correct.rs.diff:4:}}
```

Here, we instead define a single task `Test` which is an `enum` with two variants.
In its `Task` implementation, we match ourselves and return `"Hello World!"` when the variant is `ReturnHelloWorld`.
When the variant is `ReturnHelloWorld`, we require `&Self::ReturnHelloWorld` through the context, which is now valid because it is an instance of `Test`, and turn its string lowercase and return that.
This now works due to only having a single task type.
Run the test with `cargo test` to confirm it is working.

```admonish info title="Rust Help" collapsible=true
[Enums](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html) define a type by a set of variants, similar to enums in other languages, sometimes called tagged unions in other languages.
The `match` expression matches the variant and dispatches based on that, similar to switch statements in other languages.
```

We have defined the API for the build system and implemented a non-incremental version of it.
We're now ready to start implementing an incremental context in the next chapter.
