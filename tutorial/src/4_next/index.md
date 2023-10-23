# What's Next?

## Future work

I'd still like to write a tutorial going over an example where we use this build system for incremental batch builds, but at the same time also reuse the same build for an interactive environment.
This example will probably be something like interactively developing a parser with live feedback.
 
I'd also like to go over all kinds of extensions to the build system, as there are a lot of interesting ones.
Unfortunately, those will not be guided like the rest of this programming tutorial, due to lack of time.

## PIE implementations

In this tutorial, you implemented a large part of the PIE, the programmatic incremental build system that I developed during my PhD.
There are currently two versions of PIE:

- [PIE in Java](https://github.com/metaborg/pie). The motivation for using Java was so that we could use PIE to correctly incrementalize the [Spoofax Language Workbench](https://spoofax.dev/), a set of tools and interactive development environment (IDE) for developing programming languages. In Spoofax, you develop a programming language by _defining_ the aspects of your language in _domain-specific meta-languages_, such as SDF3 for syntax definition, and Statix for type system and name binding definitions. 
 
  Applying PIE to Spoofax culminated in [Spoofax 3](https://spoofax.dev/spoofax-pie/develop/) (sometimes also called Spoofax-PIE), a new version of Spoofax that uses PIE for all tasks such as generating parsers, running parsers on files to create ASTs, running highlighters on those ASTs to provide syntax highlighting for code editors, etc. Because all tasks are PIE tasks, we can do correct and incremental batch builds of language definitions, but also live development of those language definitions in an IDE, using PIE to get feedback such as inline errors and syntax highlighting as fast as possible.
- [PIE in Rust](https://github.com/Gohla/pie), a superset of what you have been developing in this tutorial. I plan to make this a full-fledged and usable library for incremental batch builds and interactive systems. You are of course free to continue developing the library you made in this tutorial, but I would appreciate users and/or contributions to the PIE library!

  The motivation for developing a PIE library in Rust was to test whether the idea of a programmatic incremental build system really is programming-language agnostic, as a target for developing this tutorial, and to get a higher-performance implementation compared to the Java implementation of PIE. 

  In my opinion, implementing PIE in Rust as part of this tutorial is a much nicer experience than implementing it in Java, due to the more powerful type system and great tooling provided by Cargo. However, supporting multiple task types, which we didn't do in this tutorial, is a bit of a pain due to requiring trait objects, which can be really complicated to work with in certain cases. In Java, everything is a like a trait object, and you get many of these things for free, at the cost of garbage colletion and performance of course.

## Publications about Programmatic Incremental Build System and PIE

```admonish warning title="Under construction"
This subsection is under construction.
```

### My publications

I wrote two papers about programmatic incremental build systems, and PIE, which you can [find the most updated versions of in my dissertation](https://gkonat.github.io/assets/dissertation/konat_dissertation.pdf).
I wrote these papers during my time as a PhD candidate at the [Programming Languages group](https://pl.ewi.tudelft.nl) at [Delft University of Technology](https://www.tudelft.nl/).

The two papers are found in my dissertation at:
- Chapter 7, page 83: PIE: A Domain-Specific Language for Interactive Software Development Pipelines. 
 
  This describes a domain-specific language (DSL) for programmatic incremental build systems, and introduces the PIE library in Kotlin. This implementation was later changed to a pure Java library to reduce the number of dependencies. 
- Chapter 8, page 109: Scalable Incremental Building with Dynamic Task Dependencies.

  This describes a hybrid incremental build algorithm that builds from the bottom-up, only switching to top-down building when necessary. Bottom-up builds are more efficient with changes that have a small effect (i.e., most changes), due to only _checking the part of the dependency graph affected by changes_. Therefore, this algorithm _scales down to small changes while scaling up to large dependency graphs_. 

  Unfortunately, we did not implement (hybrid) bottom-up building in this tutorial due to a lack of time. However, the [PIE in Rust](https://github.com/Gohla/pie) library has a [bottom-up context implementation](https://github.com/Gohla/pie/blob/master/pie/src/context/bottom_up.rs) which you can check out. Due to similarities between the top-down and bottom-up context, some common functionality was [extracted into an extension trait](https://github.com/Gohla/pie/blob/master/pie/src/context/mod.rs).

### Supervised publications

TODO: observability

TODO: improved PIE DSL

TODO: Stratego compiler

### Pluto

PIE is based on [Pluto, a programmatic incremental build system](https://www.pl.informatik.uni-mainz.de/files/2019/04/pluto-incremental-build.pdf) developed by Sebastian Erdweg et al.
This is not a coincidence, as Sebastian Erdweg frequently contributed improvements to our software (Spoofax 2 uses Pluto), he was my PhD copromotor, and coauthored the "Scalable Incremental Building with Dynamic Task Dependencies" paper.

The Pluto paper provides a more formal proof of incrementality and correctness for the top-down build algorithm, which provides confidence that this algorithm works correctly, but also explains the intricate details of the algorithm very well.
Note that Pluto uses "builder" instead of "task".
In fact, a Pluto builder is more like an incremental function that _does not carry its input_, whereas a PIE task is more like an incremental closure that includes its input.
 
PIE uses almost the same top-down build algorithm as Pluto, but there are some technical changes that make PIE more convenient to use.
In Pluto, tasks are responsible for storing their output and dependencies, called "build units", which are typically stored in files.
In PIE, the library handles this for you, which is convenient, but does require a mapping from a `Task` (using its `Eq` and `Hash` impls) to its dependencies and output, which is what the `Store` does.

### Other related work

