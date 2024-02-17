use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Context, OutputChecker, Task, ValueEq};

/// [Task output checker](OutputChecker) that checks by equality.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct EqualsChecker;
impl<O: ValueEq> OutputChecker<O> for EqualsChecker {
  type Stamp = O;
  #[inline]
  fn stamp(&self, output: &O) -> Self::Stamp { output.clone() }

  #[inline]
  fn check<'i>(&self, output: &'i O, stamp: &'i Self::Stamp) -> Option<Box<dyn Debug + 'i>> {
    if output != stamp {
      Some(Box::new(output)) // TODO: don't box
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
  #[inline]
  fn stamp(&self, _output: &O) -> Self::Stamp { () }

  #[inline]
  fn check<'i>(&self, _output: &'i O, _stamp: &'i Self::Stamp) -> Option<Box<dyn Debug + 'i>> {
    None::<Box<dyn Debug>>
  }
}


/// Implement task for `()` that does nothing and just returns `()`.
impl Task for () {
  type Output = ();
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output { () }
}

/// Implement task for [`Box`] wrapped tasks.
impl<T: Task> Task for Box<T> {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}
/// Implement task for [`Rc`] wrapped tasks.
impl<T: Task> Task for Rc<T> {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}
/// Implement task for [`Arc`] wrapped tasks.
impl<T: Task> Task for Arc<T> {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().execute(context)
  }
}
