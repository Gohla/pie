# Build your own Programmatic Incremental Build System

## Requirements

Install mdBook and several plugins:

```shell
cargo install mdbook mdbook-admonish mdbook-external-links
```

## Building

To test all the code fragments and generate diffs in `stepper/out` which the tutorial uses, run:

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
