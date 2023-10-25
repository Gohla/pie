# Setup

## Rust

We will be using [Rust](https://www.rust-lang.org/) to implement a programmatic incremental build system.
Therefore, first make sure [Rust is installed](https://www.rust-lang.org/tools/install).

If you already have Rust installed, make sure to update to at least Rust 1.65.0 (Nov 2022), as we're using features only available from that release.

Once Rust is installed, you should have access to [cargo](https://doc.rust-lang.org/cargo/).
Cargo is Rust's package manager but also the command-line tool that ties everything together.
Verify your Rust installation by running `cargo` from the command-line, which will show the help page for the cargo command.

## Rust Editor / IDE

This tutorial does not require a specific Rust editor or IDE.
All you need is some way to edit files, and some way to run `cargo`.

If you like to work in a terminal, [rust-analyzer](https://rust-analyzer.github.io/), the primary Language Server Protocol (LSP) implementation of Rust, can be used in [Emacs](https://rust-analyzer.github.io/manual.html#emacs) and [Vim/Neovim](https://rust-analyzer.github.io/manual.html#vimneovim).

If you prefer a graphical editor, a popular choice is [Visual Studio Code](https://code.visualstudio.com/) with the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) plugin.

Personally, I am using the [RustRover](https://www.jetbrains.com/rust/) IDE, as it provides (in my opinion) the most polished and feature-full Rust editor/IDE.
At the time of writing, RustRover is still in early access preview, meaning that it is free to use, but is still technically beta software.
However, it is very stable despite being in preview.
Once RustRover comes out of preview, you will need to pay for it though (or apply for a [free educational](https://www.jetbrains.com/community/education/#students/) or [open source](https://www.jetbrains.com/community/opensource/#support) license).

## Creating a new Rust project

In this tutorial, we will create a subset of the [PIE in Rust](https://github.com/Gohla/pie) library, so we want to create a Rust library called `pie`.
However, later on in the tutorial we will also create an additional library for unit testing utilities, so we need to set up a Rust project that supports multiple libraries.

Therefore, first create a `programmatic-builds` directory, which will serve as the root directory of the project.
This does not have to be called `programmatic-builds`, you can use a different name.

```admonish warning title="Important"
In the rest of the tutorial, we assume that you are in your `programmatic-builds` root directory.
So if you are instructed to create files or directories, they are always relative to your `programmatic-builds` root directory!
```

Now let's set up the `pie` library.
Create the `pie` directory, and then create the `pie/Cargo.toml` file with the following contents:

```toml,
{{#include Cargo.toml}}
```

Then create the `pie/src` directory and create the `pie/src/lib.rs` file, which will be left empty for now.

This marks `pie` as a Rust library, with version "0.1.0" and using Rust edition 2021.
The directory structure should look as follows (inside your `programmatic-builds` root directory):

```
{{#include ../../gen/0_intro/1_setup/dir.txt}}
```

Now we can build the project to see if it was set up correctly.
Open up a terminal, go into the `pie` directory, and run `cargo build`.
If all is well, the output should look something like:

```shell,
{{#include ../../gen/0_intro/1_setup/cargo.txt}}
```

```admonish warning title="Important"
In the rest of the tutorial, if you are instructed to run `cargo` commands, always run them inside the `pie` directory!
```

If you're using a Rust editor or IDE, it probably also has a mechanism for running cargo on your project.
You can of course use that in place of running cargo from a terminal. 

```admonish tip title="Rust Editions" collapsible=true
[Rust editions](https://doc.rust-lang.org/edition-guide/introduction.html) enable new features as an opt-in, without breaking existing code.
We use [Rust edition 2021](https://doc.rust-lang.org/edition-guide/rust-2021/index.html), which is the latest edition at the time of writing.
```

## Source control (optional)

I recommend storing your code in a source control system such as [Git](https://git-scm.com/), and uploading it to a source code hub such as [GitHub](https://github.com/).
A source control system allows you to look at changes and to go back to older versions, and uploading to a source code hub then provides a convenient backup.

If you use Git, create the `pie/.gitignore` file with:

```.gitignore
/target
```

This ignores the `target` directory that Rust/Cargo uses to store intermediate and binary files.

Continue to the next chapter where we will start implementing the "programmatic" part of programmatic incremental build systems.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/0_intro/1_setup/source.zip).
```
