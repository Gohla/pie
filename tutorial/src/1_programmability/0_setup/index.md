# Setup

```admonish warning title="Under construction"
This page was quickly created to make setup possible, but is unfinished.
```

Make sure [Rust is installed](https://www.rust-lang.org/tools/install).

We start by creating a new Rust crate.
Create the `pie` directory and create the `pie/Cargo.toml` file with the following contents:

```toml,
{{#include Cargo.toml}}
```

Then create the `pie/src` directory and create the `pie/src/lib.rs` file, which will be left empty for now.

Run `cargo build` to test if the project was set up correctly.
The output should look something like:

```shell,
{{#include ../../gen/1_programmability/0_setup/cargo.txt}}
```

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/1_programmability/0_setup/source.zip).
```
