# Programmability

In the introduction, we saw an example with _tasks_ being the unit of incremental computation, and the _context_ providing a way to create dynamic dependencies that enable incrementality.
Those two concepts, `Task` and `Context`, are the core of a programmatic incremental build system which we will implement in this chapter.

We will continue with the following steps in the next two sections:

1) Create the `Task` and `Context` API, forming the core of the build system.
2) Create a non-incremental `Context` implementation and test it against `Task` implementations, to get a feeling for the API without the complexities of incrementality.
