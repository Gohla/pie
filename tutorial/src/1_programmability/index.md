# Programmability

[//]: # (## Dynamic Dependencies)

[//]: # ()
[//]: # (To achieve these properties, our build system will need to work with _dynamic dependencies_.)

[//]: # (But let's first understand _static dependencies_, which is what most build system)

[//]: # (Most build systems work with _static dependencies_.)

[//]: # (That is, dependencies have to be defined up front, in the build script.)

[//]: # (For example, consider the [Make]&#40;https://www.gnu.org/software/make/&#41; target `foo.o: foo.c bar.h ; gcc foo.c`.)

[//]: # (It states that in order to make `foo.o`, we call `gcc foo.c` which requires the `foo.c` and `bar.h` files.)

[//]: # (Here, the dependencies are stated statically in the build script.)
