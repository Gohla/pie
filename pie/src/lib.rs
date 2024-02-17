//! # Trait bounds
//!
//! [`Task`] and [`Resource`] are bounded by [`Key`] so that we can store types of those traits as a key in a
//! hashmap in trait object form. We need to store these types under a trait object to support arbitrary task and
//! resource types. We also need to store an additional clone for a reverse hashmap.
//!
//! [`OutputChecker`] and [`ResourceChecker`] are also bounded by [`Key`], because types of these traits may be
//! used as values in tasks, which would require them to be bounded by [`Key`] anyway. This reduces boilerplate in
//! tasks that are generic over [`OutputChecker`] and [`ResourceChecker`], as the [`Key`] bound can be omitted.
//!
//! [`Task::Output`], [`OutputChecker::Stamp`], and [`ResourceChecker::Stamp`] are bounded by [`Value`] because
//! we need to store (cache) these values. We need to store these values for an indeterminate time, so non-`'static`
//! references are ruled out. We need to clone outputs to store them. When checking dependencies, we need to clone the
//! dependencies (due to lifetime/borrow complications). Since these types are used in dependencies, that is another
//! reason they need to be [`Clone`].
//!
//! All user-implementable traits and corresponding associated types are bounded by [`Debug`] for debugging/logging.

use std::any::Any;
use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;

use crate::tracker::Tracker;
use crate::trait_object::KeyObj;

pub mod task;
pub mod resource;
pub mod tracker;
#[macro_use]
pub mod trait_object;

mod pie;
mod context;
mod store;
mod dependency;

/// Trait alias for types that are used as values: types that can be cloned, debug formatted, and contain no
/// non-`'static` references.
pub trait Value: Clone + Debug + 'static {}
impl<T: Clone + Debug + 'static> Value for T {}

/// Trait alias for types that are used as equatable values: types that are [values](Value) and that can be equality
/// compared.
pub trait ValueEq: Value + Eq {}
impl<T: Value + Eq> ValueEq for T {}

/// Trait alias for types that are used as keys: types that are [equatable values](ValueEq) and that can be hashed.
pub trait Key: ValueEq + Hash {}
impl<T: ValueEq + Hash> Key for T {}

/// A unit of computation in a programmatic incremental build system.
pub trait Task: Key {
  /// Type of task outputs.
  type Output: Value;

  /// Execute the task under `context`, returning an output.
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}

/// Programmatic incremental build context, enabling tasks to require other tasks and read/write from/to resources,
/// programmatically creating precise dynamic dependencies that are used to incrementally execute tasks.
///
/// Tasks can [require](Self::require) other tasks, creating a task dependency and getting their consistent (i.e.,
/// most up-to-date) output value.
///
/// Tasks can [read](Self::read) from a [resource](Resource), creating a resource read dependency and getting a reader
/// for reading from the resource. Subsequently, tasks can [write](Self::write) to a [resource](Resource), first writing
/// to the resource through a writer, then creating a resource write dependency.
///
/// When a dependency of the task is inconsistent, that task is inconsistent and will be re-executed when required.
///
/// This trait is *not* intended to be user-implementable.
pub trait Context {
  /// Requires `task` using `checker` for consistency checking, creating a task dependency and returning its consistent
  /// (i.e., most up-to-date) output value.
  fn require<T, H>(&mut self, task: &T, checker: H) -> T::Output where
    T: Task,
    H: OutputChecker<T::Output>;

  /// Creates a read dependency to `resource` using `checker` for consistency checking, then returns a
  /// [reader](Resource::Reader) for reading the resource.
  fn read<T, R, H>(&mut self, resource: &T, checker: H) -> Result<R::Reader<'_>, H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>;
  /// Creates a [writer](Resource::Writer) for `resource`, runs `write_fn` with that writer, then creates a write
  /// dependency to `resource` using `checker` for consistency checking.
  fn write<T, R, H, F>(&mut self, resource: &T, checker: H, write_fn: F) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
    F: FnOnce(&mut R::Writer<'_>) -> Result<(), R::Error>;

  /// Creates a [writer](Resource::Writer) for `resource`. This does *not* create a dependency. After writing to the
  /// resource, call [written_to](Self::written_to) to create the dependency.
  ///
  /// Prefer [write](Self::write) if possible, as it handles writing and creating the dependency in one call.
  fn create_writer<'r, R>(&'r mut self, resource: &'r R) -> Result<R::Writer<'r>, R::Error> where
    R: Resource;
  /// Creates a write dependency to `resource` using `checker` for consistency checking.
  fn written_to<T, R, H>(&mut self, resource: &T, checker: H) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>;
}

/// Consistency checker for task outputs of type `O`, producing and checking output stamps. For example, the
/// [equals checker](task::EqualsChecker) uses the output of a task as stamp, and checks whether they are equal.
pub trait OutputChecker<O>: Key {
  /// Type of stamps returned from [stamp](Self::stamp).
  type Stamp: Value;
  /// Stamps `output` into a [`Stamp`](Self::Stamp).
  fn stamp(&self, output: &O) -> Self::Stamp;

  /// Checks whether `output` is inconsistent w.r.t. `stamp`, returning `Some(inconsistency)` if inconsistent, `None` if
  /// consistent. The returned inconsistency can be used for debugging purposes, such as logging what has changed.
  fn check<'i>(&self, output: &'i O, stamp: &'i Self::Stamp) -> Option<Box<dyn Debug + 'i>>;
}


/// A resource representing global (mutable) state, such as a path identifying a file on a filesystem.
pub trait Resource: Key {
  /// Type of readers returned from [read](Self::read), with `'rs` representing the lifetime of the resource state.
  type Reader<'rs>;
  /// Type of writers returned from [write](Self::write), with `'r` representing the lifetime of this resource.
  type Writer<'r>;
  /// Type of errors returned from all methods.
  type Error: Error;

  /// Creates a reader for this resource, with access to global mutable [resource `state`](ResourceState).
  fn read<'rs, RS: ResourceState<Self>>(&self, state: &'rs mut RS) -> Result<Self::Reader<'rs>, Self::Error>;
  /// Creates a writer for this resource, with access to global mutable [resource `state`](ResourceState).
  fn write<'r, RS: ResourceState<Self>>(&'r self, state: &'r mut RS) -> Result<Self::Writer<'r>, Self::Error>;
}

/// Provides access to global mutable state for [resources](Resource) of type `R`. Each unique resource type `R` has
/// access to one value that can be of any type that implements [`Any`] (i.e., all types without non-`'static`
/// references).
///
/// This trait is *not* intended to be user-implementable.
pub trait ResourceState<R> {
  /// Gets the state as `S`. Returns `Some(&state)` if the state of type `S` exists, `None` otherwise.
  fn get<S: Any>(&self) -> Option<&S>;
  /// Gets the mutable state as `S`. Returns `Some(&mut state)` if the state of type `S` exists, `None` otherwise.
  fn get_mut<S: Any>(&mut self) -> Option<&mut S>;
  /// Sets the `state`.
  fn set<S: Any>(&mut self, state: S);

  /// Gets the boxed state. Returns `Some(&state)` if the state exists, `None` otherwise.
  fn get_boxed(&self) -> Option<&Box<dyn Any>>;
  /// Gets the mutable boxed state. Returns `Some(&mut state)` if the state exists, `None` otherwise.
  fn get_boxed_mut(&mut self) -> Option<&mut Box<dyn Any>>;
  /// Sets the boxed `state`.
  fn set_boxed(&mut self, state: Box<dyn Any>);

  /// Gets the state as `S` or sets a default. If no state was set, or if it is not of type `S`, first sets the state to
  /// `S::default()`. Then returns the state as `&state`.
  fn get_or_set_default<S: Default + Any>(&mut self) -> &S;
  /// Gets the mutable state as `S` or sets a default. If no state was set, or if it is not of type `S`, first sets the
  /// state to `S::default()`. Then returns the state as `&mut state`.
  fn get_or_set_default_mut<S: Default + Any>(&mut self) -> &mut S;
}

/// Consistency checker for resources, producing and checking resource stamps. For example, for filesystem resources, a
/// last modified checker creates last modified stamps and checks whether they have changed, and a hash checker creates
/// file content hash stamps and checks whether they have changed.
pub trait ResourceChecker<R: Resource>: Key {
  /// Type of stamps returned from stamp methods.
  type Stamp: Value;
  /// Type of errors returned from all methods.
  type Error: Error;

  /// Stamp `resource` with access to `state`.
  fn stamp<RS: ResourceState<R>>(&self, resource: &R, state: &mut RS) -> Result<Self::Stamp, Self::Error>;
  /// Stamps `resource` with a `reader` for that resource.
  ///
  /// The `reader` is fresh: it is first passed to this checker before being passed to a task. However, because it is
  /// later passed to a task, `reader` must be _left in a fresh state after stamping_. For example, a
  /// [buffered reader](std::io::BufReader) must be [rewound](std::io::Seek::rewind) after using it.
  fn stamp_reader(&self, resource: &R, reader: &mut R::Reader<'_>) -> Result<Self::Stamp, Self::Error>;
  /// Stamps `resource` with a `writer` for that resource.
  ///
  /// The `writer` is dirty: it was first used by a task to write data. Therefore, be sure to restore the `writer` to a
  /// fresh state before reading from it. For example, a [file](std::fs::File) must be [rewound](std::io::Seek::rewind)
  /// before using it.
  ///
  /// There is no guarantee that `resource` still exists, as it may have been removed by a task. Therefore, `writer` may
  /// contain stale metadata for certain resources. For example, a [file](std::fs::File) can be
  /// [removed](std::fs::remove_file), but its [file](std::fs::File) descriptor will still return cached (stale)
  /// [metadata](std::fs::File::metadata). If that can be the case, be sure to check whether `writer` is still
  /// consistent with `resource`.
  fn stamp_writer(&self, resource: &R, writer: R::Writer<'_>) -> Result<Self::Stamp, Self::Error>;

  /// Checks whether `resource` is inconsistent w.r.t. `stamp`, with access to `state`. Returns `Some(inconsistency)`
  /// when inconsistent, `None` when consistent. The returned inconsistency can be used for debugging purposes, such as
  /// logging what has changed.
  fn check<RS: ResourceState<R>>(
    &self,
    resource: &R,
    state: &mut RS,
    stamp: &Self::Stamp,
  ) -> Result<Option<impl Debug>, Self::Error>;

  /// Wraps a [resource `error`](Resource::Error) into [`Self::Error`].
  fn wrap_error(&self, error: R::Error) -> Self::Error;
}


/// Main entry point into PIE, a sound and incremental programmatic build system.
#[repr(transparent)]
pub struct Pie<A>(pie::PieInternal<A>);

impl Default for Pie<()> {
  fn default() -> Self {
    Self(pie::PieInternal::default())
  }
}

impl<A: Tracker> Pie<A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self {
    Self(pie::PieInternal::with_tracker(tracker))
  }

  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  #[inline]
  pub fn new_session(&mut self) -> Session {
    self.0.new_session()
  }
  /// Runs `f` inside a new build session.
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session) -> R) -> R {
    self.0.run_in_session(f)
  }

  /// Gets the [tracker](Tracker).
  #[inline]
  pub fn tracker(&self) -> &A {
    self.0.tracker()
  }
  /// Gets the mutable [tracker](Tracker).
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A {
    self.0.tracker_mut()
  }

  /// Gets the [resource state](ResourceState) for [resource](Resource) type [`R`].
  #[inline]
  pub fn resource_state<R: Resource>(&self) -> &impl ResourceState<R> {
    self.0.resource_state()
  }
  /// Gets the mutable [resource state](ResourceState) for [resource](Resource) type [`R`].
  #[inline]
  pub fn resource_state_mut<R: Resource>(&mut self) -> &mut impl ResourceState<R> {
    self.0.resource_state_mut()
  }
}

/// A session in which builds are executed.
#[repr(transparent)]
pub struct Session<'p>(pie::SessionInternal<'p>);
impl<'p> Session<'p> {
  /// Requires `task`, returning its consistent output.
  #[inline]
  pub fn require<T: Task>(&mut self, task: &T) -> T::Output {
    self.0.require(task)
  }

  /// Creates a bottom-up build. Call [schedule_tasks_affected_by](BottomUpBuild::schedule_tasks_affected_by) for each
  /// changed resource to schedule tasks affected by changed resources.
  ///
  /// Then call [update_affected_tasks](BottomUpBuild::update_affected_tasks) to update all affected tasks in a
  /// bottom-up build.
  ///
  /// Finally, use [require](Self::require) of this session to get up-to-date task outputs if needed.
  #[inline]
  #[must_use]
  pub fn create_bottom_up_build<'s>(&'s mut self) -> BottomUpBuild<'p, 's> {
    BottomUpBuild(self.0.create_bottom_up_build())
  }

  /// Gets all errors produced during dependency checks.
  #[inline]
  #[must_use]
  pub fn dependency_check_errors(&self) -> impl Iterator<Item=&dyn Error> + ExactSizeIterator {
    self.0.dependency_check_errors()
  }
}

#[repr(transparent)]
pub struct BottomUpBuild<'p, 's>(pie::BottomUpBuildInternal<'p, 's>);
impl<'p, 's> BottomUpBuild<'p, 's> {
  /// Schedule tasks affected by `resource`.
  #[inline]
  pub fn schedule_tasks_affected_by(&mut self, resource: &dyn KeyObj) {
    self.0.schedule_tasks_affected_by(resource);
  }
  /// Update all tasks affected by resource changes.
  #[inline]
  pub fn update_affected_tasks(self) {
    self.0.update_affected_tasks();
  }
}
