# Programmable Build System API

In this first chapter, we will program the core API of the programmatic incremental build system, and implement an extremely simple non-incremental version of the build system to get started.


## Core API

The unit of computation in a programmatic build system is a *task*.
A task is kind of like a closure: a value that can be executed to produce their output.
However, in an *incremental* programmatic build system, we also need to keep track of *dynamic dependencies* that are made while tasks are executing.
Therefore, tasks are executed under a *context* which enable them to create these dependencies.
Tasks *require* other tasks through the context, creating a dynamic dependency and returning their up-to-date output.

On the other hand, an incremental context wants to *selectively execute tasks* — only those that are affected by a change.
To that end, a context will selectively execute tasks, tasks require other tasks through the context, the context selectively executes those, and so forth.
Thus, tasks and contexts are mutually recursive.
Let's make this more concrete by defining tasks and contexts in code.

Since we want users of the build system to implement their own tasks, we will define `Task` as a trait.
Likewise, we will also be implementing multiple contexts in this tutorial, so we will also define `Context` as a trait.
Add the following code to your `src/lib.rs` file:

```rust
{{#include lib.rs}}
```

```admonish
If this seems overwhelming to you, don't worry. We will go through the API and explain things. But more importantly, the API should become more clear once we implement it in the next section and subsequent chapters.
Furthermore, if you're new to Rust and/or need help understanding certain concepts, I will try to explain them in Rust Help blocks. They are collapsed by default to reduce distraction, clicking the header opens them. See the first Rust Help block at the end of this section.
```

The `Task` trait has several supertraits that we will need later in the tutorial to implement incrementality:

* `Eq` and `Hash`: to check whether a task is equal to another one, and to create a hash of it, so we can use a `HashMap` to get the output of a task if it is up-to-date.
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

Note the mutual definition of `Task` and `Context`, which we discussed earlier.

[//]: # (Note that a `Context` is generic over `Task` `T`, meaning that a context can work with any task implementation.)

[//]: # (However, the `Task::execute` method is generic over any `Context` with `T` set to `Self`, so a task can only work with contexts that work with its own specific task type.)

[//]: # (This limits us to only supporting one type of task, because)

[//]: # ()
[//]: # (Finally, it is important to note that `Context` is only generic over *one type of task*: you can't combine a `Context<CopyFile>` with `Context<WriteToFile>`.)

[//]: # (While this is quite limiting, we do this to keep the tutorial simple, and we can work around it by defining )

```admonish info title="Rust Help" collapsible=true
[The Rust Programming Language](https://doc.rust-lang.org/book/ch00-00-introduction.html) is an introductory book about Rust. I will try to provide links to the book where possible.

Rust has a [module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) for project organization. The `lib.rs` file is the "main file" of a library. Later on, we will be creating more modules in different files.

Things are imported into the current scope with [`use`](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html) statements. We import the `Debug` and `Hash` traits from the standard library with two `use` statements. `::` is used for nesting, similar to namespaces in C++.

Rust models the concept of [ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html) to enable memory safety without a garbage collector.
The `execute` method accepts a *reference* to the current type, indicated with `&`: `&self`. This reference is *immutable*, meaning that we can read data from it, but not mutate it. In Rust, things are immutable by default.
On the other hand, `execute` accepts a *mutable reference* to the context, indicated with `&mut`: `context: &mut C`, which does allow mutation.

[Traits](https://doc.rust-lang.org/book/ch10-02-traits.html) are the main mechanism for open extensibility in Rust. They are comparable to interfaces in class-oriented languages. We will implement a task and context in the next section.

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

```rust
{{#include non_incremental.rs}}
```

```rust
{{#include non_incremental_test_1.rs}}
```

```rust,customdiff
{{#include ../../stepper/out/non_incremental_test_2.rs.diff:4:}}
```
