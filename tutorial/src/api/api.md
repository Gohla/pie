# Programmable Build System API

The unit of computation in a programmatic build system is a *task*.
A task is kind of like a closure: a value that can be executed to produce their output.
However, in an *incremental* programmatic build system, we also need to keep track of *dynamic dependencies* that are made while tasks are executing.
Therefore, tasks are executed under a *context* which enable them to create these dependencies.
Tasks *require* other tasks through the context, creating a dynamic dependency and returning their up-to-date output.

On the other hand, an incremental context wants to *selectively execute tasks* -- only those that are affected by a change.
To that end, a context will selectively execute tasks, tasks require other tasks through the context, the context selectively executes those, and so forth.
Thus, tasks and contexts are mutually recursive.

Let's make this more concrete by defining tasks and contexts in code.
Since we want users of the build system to implement their own tasks, we will define `Task` as a trait.
Likewise, we will also be implementing multiple contexts in this tutorial, so we will also define `Context` as a trait.
Add the following code to your `src/lib.rs` file:

```rust
{{#include lib.rs}}
```

If you're new to Rust and/or need help understanding certain concepts, I will try to explain them in Rust Help blocks. They are collapsed by default to reduce distraction, clicking the header opens them. See the first Rust Help block below.

The `Task` trait has several supertraits that we will need later in the tutorial to implement incrementality:

* `Eq` and `Hash`: to check whether a task is equal to another one, and to create a hash of it, so we can use a `HashMap` to get the output of a task if it is up-to-date.
* `Clone`: to create a clone of the task so that we can store it in the `HashMap` without having ownership of it.
* `Debug`: to format the task for debugging purposes.

A `Task` has a single method `execute`, which takes a reference to itself (`&self`), and a mutable reference to a context (`context: &mut C`), and produces a value of type `Self::Output`.
Because `Context` is a trait, we use generics (`<C: Context<Self>>`) to have `execute` work for any `Context` implementation.
The `execute` method takes self by reference such that a task can access its data, but not modify it, as that could throw off incrementality by changing the hash/equality of the task.

The type of output of a task is defined by the `Output` associated type, and this type must implement `Clone`, `Eq`, and `Debug` for the same reason as `Task`.




```admonish info title="Rust Help" collapsible=true

[trait](https://doc.rust-lang.org/book/ch10-02-traits.html) (like an *interface* in class-oriented programming languages).
[method](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
associated type
`Task` is defined with `pub` (public) [visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html), such that users of the library can implement it. 
```
