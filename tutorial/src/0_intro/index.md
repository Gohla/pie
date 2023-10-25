# Build your own Programmatic Incremental Build System

This is a programming tutorial where you will build your own _programmatic incremental build system_ in [Rust](https://www.rust-lang.org/).

The primary goal of this tutorial is to provide understanding of programmatic incremental build systems through implementation and experimentation.

Although the tutorial uses Rust, you _don't_ need to be a Rust expert to follow it.
A secondary goal of this tutorial is to teach more about Rust through implementation and experimentation, given that you already have programming experience (in another language) and are willing to learn. 
Therefore, all Rust code is available, and I try to explain and link to the relevant Rust book chapters as much as possible.

This is of course not a full tutorial or book on Rust.
For that, I can recommend the excellent [The Rust Programming Language](https://doc.rust-lang.org/book/) book.
However, if you like to learn through examples and experimentation, or already know Rust basics and want to practice, this might be a fun programming tutorial for you!

[//]: # ()
[//]: # (Another secondary goal is to show what I think are several good software writing practices, such as dividing code into modules, thinking about what to expose as API, writing unit and integration tests, etc.)

[//]: # (Where possible I will try to explain design decisions, discuss tradeoffs, or provide more info about optimizations.)

We will first motivate programmatic incremental build systems.

## Motivation

A programmatic incremental build system is a mix between an incremental build system and an incremental computation system, with the following key properties:

- _Programmatic_: Build scripts are regular programs written in a programming language, where parts of the build script implement an API from the build system. This enables build authors to write incremental builds with the full expressiveness of the programming language.
- _Incremental_: Builds are truly incremental -- only the parts of a build that are affected by changes are executed.
- _Correct_: Builds are fully correct -- all parts of the build that are affected by changes are executed. Builds are free of glitches: only up-to-date (consistent) data is observed.
- _Automatic_: The build system takes care of incrementality and correctness. Build authors _do not_ have to manually implement incrementality. Instead, they only have to explicitly _declare dependencies_.
- _Multipurpose_: The same build script can be used for incremental batch builds in a terminal, but also for live feedback in an interactive environment such as an IDE. For example, a compiler implemented in this build system can provide incremental batch compilation but also incremental editor services such as syntax highlighting or code completion.

#### Teaser Toy Example

As a small teaser, here is a simplified version of a programmatic incremental toy build script that copies a text file by reading and writing:

```rust
struct ReadFile { file: PathBuf }
impl Task for ReadFile {
  fn execute<C: Context>(&self, context: &mut C) -> Result<String, io::Error> {
    context.require_file(&self.file)?;
    fs::read_to_string(&self.file)
  }
}

struct WriteFile<T> { task: T, file: PathBuf }
impl<T: Task> Task for WriteFile<T> {
  fn execute<C: Context>(&self, context: &mut C) -> Result<(), io::Error> {
    let string: String = context.require_task(&self.task)?;
    fs::write(&self.file, string.as_bytes())?;
    context.provide_file(&self.file)
  }
}

fn main() {
  let read_task = ReadFile { file: PathBuf::from("in.txt") };
  let write_task = WriteFile { task: read_task, file: PathBuf::from("out.txt") };
  Pie::default().new_session().require(&write_task);
}
```

The unit of computation in a programmatic incremental build system is a _task_.
A task is kind of like a closure, a function along with its inputs that can be executed, but incremental.
For example, the `ReadFile` task carries the file path it reads from.
When we `execute` the task, it reads from the file and returns its text as a string.
However, due to incrementality, we mark the file as a `require_file` dependency through `context`, such that this task is only re-executed when the file changes!

Note that this file read dependency is created _while the task is executing_.
We call these _dynamic dependencies_.
This is one of the main benefits of programmatic incremental build systems: you create dependencies _while the build is executing_, instead of having to declare them upfront!

Dynamic dependencies are also created between tasks.
For example, `WriteFile` carries a task as input, which it requires with `context.require_task` to retrieve the text for writing to a file.
We'll cover how this works later on in the tutorial.
For now, let's zoom back out to the motivation of programmatic incremental build systems.

#### Back to Motivation

I prefer writing builds in a programming language like this, over having to _encode_ a build into a YAML file with underspecified semantics, and over having to learn and use a new build scripting language with limited tooling.
By _programming builds_, I can reuse my knowledge of the programming language, I get help from the compiler and IDE that I'd normally get while programming, I can modularize and reuse parts of my build as a library, and can use other programming language features such as unit testing, integration testing, benchmarking, etc.

Programmatic builds _do not exclude declarativity_, however.
You can layer declarative features on top of programmatic builds, such as declarative configuration files that determine _what_ should be built without having to specify _how_ things are built.
For example, you could write a task like the one from the example, which reads and parses a config file, and then dispatch tasks that build required things.
Therefore, programmatic builds are useful for both small one-off builds, and for creating larger incremental build systems that work with a lot of user inputs.

Dynamic dependencies enable creating precise dependencies, _without requiring staging_, as is often found in build systems with static dependencies.
For example, dynamic dependencies in [Make](https://www.gnu.org/software/make/) requires staging: generate new makefiles and recursively execute them, which is tedious and error-prone.
[Gradle](https://gradle.org/) has a two-staged build process: first configure the task graph, then incrementally execute it.
In the execution stage, you cannot modify dependencies or create new tasks.
Therefore, more work needs to be done in the configuration stage, which is not (fully) incrementalized.
Dynamic dependencies solve these problems by doing away with staging!

Finally, precise dynamic dependencies enable incrementality but also correctness.
A task is re-executed when one or more of its dependencies become inconsistent.
For example, the `WriteFile` task from the example is re-executed when the task dependency returns different text, or when the file it writes to is modified or deleted.
This is both incremental and correct.

#### Disadvantages

Of course, programmatic incremental build systems also have some disadvantages.
These disadvantages become more clear during the tutorial, but I want to list them here to be up-front about it:

- The build system is more complicated, but hopefully this tutorial can help mitigate some of that by understanding the key ideas through implementation and experimentation.
- Some correctness properties are checked while building. Therefore, you need to test your builds to try to catch these issues before they reach users. However, I think that testing builds is something you should do regardless of the build system, to be more confident about the correctness of your build.
- More tracking is required at runtime compared to non-programmatic build systems. However, in our experience, the overhead is not excessive unless you try to do very fine-grained incrementalization. For fine-grained incrementalization, [incremental computing](https://en.wikipedia.org/wiki/Incremental_computing) approaches are more well suited.

## PIE: a Programmatic Incremental Build System in Rust

We have developed [PIE, a Rust library](https://github.com/Gohla/pie) implementing a programmatic incremental build system adhering to the key properties listed above.
It is still under development, and has not been published to crates.io yet, but it is already usable 
If you are interested in experimenting with a programmatic incremental build system, do check it out!

In this tutorial we will implement a subset of [PIE, the Rust library](https://github.com/Gohla/pie).
We simplify the internals in order to minimize distractions as much as possible, but still go over all the key ideas and concepts that make programmatic incremental build systems tick.

However, the _idea_ of programmatic incremental build systems is not limited to PIE or the Rust language.
You can implement a programmatic incremental build systems in any general-purpose programming language, or adapt the idea to better fit your preferences and/or requirements.
In fact, we first implemented [PIE in Java](https://github.com/metaborg/pie), with [PIE in Rust](https://github.com/Gohla/pie) being the second iteration, mostly simplifying internals to make it easier to explain.

For a more thorough discussion of future, existing, and related work, see [the last chapter of this book](../4_next/index.md).

## Feedback & Contributing

This tutorial is open source, hosted at <https://github.com/Gohla/pie>.
If you find an error in the code or text of this tutorial, or want to report other kinds of problems, you can report it on the [issue tracker](https://github.com/Gohla/pie/issues).
Small fixes can be sent as a pull request by pressing the edit button in the top-right corner.

Let's continue with the tutorial.
The next section covers installing Rust and setting up a fresh Rust project.
