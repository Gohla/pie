# Build your own Programmatic Incremental Build System

## Requirements

Install mdBook and several plugins:

```shell
cargo install mdbook
cargo install mdbook-admonish
cargo install mdbook-external-links
```

## Building

To build the tutorial once, run:

```shell
mdbook build
```

To interactively build, run:

```shell
mdbook serve
```

To test all the code fragments, run:

```shell
cd stepper
cargo run
```
