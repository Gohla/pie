# Build your own Programmatic Incremental Build System

A tutorial on building your own programmatic incremental build system, aiming to teach the concepts of PIE.
Live hosted version at: <https://gohla.github.io/pie/>

## Requirements

Install mdBook and several plugins:

```shell
cargo install mdbook mdbook-admonish mdbook-external-links
```

## Building

To test all the code fragments and generate outputs in `gen` which the tutorial uses, run:

```shell
cd stepper
cargo run
```

To build the tutorial once, run:

```shell
mdbook build
```

To interactively build, run:

```shell
mdbook serve
```
