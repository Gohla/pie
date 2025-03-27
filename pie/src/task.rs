use std::convert::Infallible;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Context, OutputChecker, Task, Value};

/// [Task output checker](OutputChecker) that checks by equality.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct EqualsChecker;

impl<O: Value + Eq> OutputChecker<O> for EqualsChecker {
  type Stamp = O;

  fn stamp(&self, output: &O) -> Self::Stamp {
    output.clone()
  }

  fn check(&self, output: &O, stamp: &Self::Stamp) -> Option<impl Debug> {
    if output != stamp {
      Some(output)
    } else {
      None
    }
  }
}

/// [Task output checker](OutputChecker) that checks [Ok] by equality, but [Err] only by existence.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct OkEqualsChecker;

impl<T: Value + Eq, E> OutputChecker<Result<T, E>> for OkEqualsChecker {
  type Stamp = Option<T>;

  fn stamp(&self, output: &Result<T, E>) -> Self::Stamp {
    output.as_ref().ok().cloned()
  }

  fn check(&self, output: &Result<T, E>, stamp: &Self::Stamp) -> Option<impl Debug> {
    let new_stamp = output.as_ref().ok();
    if new_stamp != stamp.as_ref() {
      Some(new_stamp)
    } else {
      None
    }
  }
}

/// [Task output checker](OutputChecker) that checks [Err] by equality, but [Ok] only by existence.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ErrEqualsChecker;

impl<T, E: Value + Eq> OutputChecker<Result<T, E>> for ErrEqualsChecker {
  type Stamp = Option<E>;

  fn stamp(&self, output: &Result<T, E>) -> Self::Stamp {
    output.as_ref().err().cloned()
  }

  fn check(&self, output: &Result<T, E>, stamp: &Self::Stamp) -> Option<impl Debug> {
    let new_stamp = output.as_ref().err();
    if new_stamp != stamp.as_ref() {
      Some(new_stamp)
    } else {
      None
    }
  }
}

/// [Task output checker](OutputChecker) that checks whether a [Result] changes from [Ok] to [Err] or vice versa.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ResultChecker;

impl<T, E> OutputChecker<Result<T, E>> for ResultChecker {
  type Stamp = bool;

  fn stamp(&self, output: &Result<T, E>) -> Self::Stamp {
    output.is_err()
  }

  fn check(&self, output: &Result<T, E>, stamp: &Self::Stamp) -> Option<impl Debug> {
    let new_stamp = output.is_err();
    if new_stamp != *stamp {
      Some(new_stamp)
    } else {
      None
    }
  }
}

/// [Task output checker](OutputChecker) that marks task dependencies as always consistent. Can be used to ignore task
/// outputs. For example, this is useful when depending on a task to write to some file which you want to read, but you
/// are not interested in the output of the task.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AlwaysConsistent;

impl<O> OutputChecker<O> for AlwaysConsistent {
  type Stamp = ();

  fn stamp(&self, _output: &O) -> Self::Stamp {
    ()
  }

  fn check(&self, _output: &O, _stamp: &Self::Stamp) -> Option<impl Debug> {
    None::<Infallible>
  }
}


/// Implement task for `()` that does nothing and just returns `()`.
impl Task for () {
  type Output = ();

  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
    ()
  }
}

/// Implement task for [`Box`] wrapped tasks.
impl<T: Task> Task for Box<T> {
  type Output = T::Output;

  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}

/// Implement task for [`Rc`] wrapped tasks.
impl<T: Task> Task for Rc<T> {
  type Output = T::Output;

  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}

/// Implement task for [`Arc`] wrapped tasks.
impl<T: Task> Task for Arc<T> {
  type Output = T::Output;

  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}
