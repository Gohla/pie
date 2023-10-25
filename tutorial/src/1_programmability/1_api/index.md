# Programmable Build System API

In this section, we will program the core API of the programmatic incremental build system.
Although we are primarily concerned with programmability in this chapter, we must design the API to support incrementality!

The unit of computation in a programmatic build system is a _task_.
A task is kind of like a closure: a value that can be executed to produce their output, but _incremental_.
To provide incrementality, we also need to keep track of the _dynamic dependencies_ that tasks make while they are executing.
Therefore, tasks are executed under an _incremental build context_, enabling them to create these dynamic dependencies.

Tasks _require_ files through the build context, creating a dynamic file dependency, ensuring the task gets re-executed when that file changes.
Tasks also _require other tasks_ through the build context, asking the build context to provide the consistent (most up-to-date) output of that task, and creating a dynamic task dependency to it.

It is then up to the build context to _check_ if it actually needs to execute that required task.
If the required task is already consistent, the build context can just return the cached output of that task.
Otherwise, the build context _executes_ the required task, caches its output, and returns the output to the requiring task.
A non-incremental context can naively execute tasks without checking.

Because tasks require other tasks through the context, and the context selectively executes tasks, the definition of task and context is mutually recursive.

```admonish abstract title="Context"
In this tutorial, we will be using the words *context*, *build context*, and *build system* interchangeably, typically using just *context* as it is concise.
```

Let's make tasks and contexts more concrete by defining them in code.

## API Implementation

Since we want users of the build system to implement their own tasks, we will define `Task` as a trait.
Likewise, we will also be implementing multiple contexts in this tutorial, so we will also define `Context` as a trait.
Add the following code to your `pie/src/lib.rs` file:

```rust,
{{#include a_api.rs}}
```

```admonish tip
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
It has a single method `require_task` for creating a dependency to a task and returning its consistent (up-to-date) result.
It takes a mutable reference to itself, enabling dynamic dependency tracking and caching, which require mutation.
Because of this, the context reference passed to `Task::execute` is also mutable.

This `Task` and `Context` API mirrors the mutually recursive definition of task and context we discussed earlier, and forms the basis for the entire build system.

```admonish note
We will implement file dependencies in the next chapter, as file dependencies only become important with incrementality.
```

Build the project by running `cargo build`.
The output should look something like:

```shell,
{{#include ../../gen/1_programmability/1_api/a_cargo.txt}}
```

In the next section, we will implement a non-incremental `Context` and test it against `Task` implementations.

```admonish tip title="Rust Help: Modules, Imports, Ownership, Traits, Methods, Supertraits, Associated Types, Visibility" collapsible=true
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

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/1_programmability/1_api/source.zip).
```
