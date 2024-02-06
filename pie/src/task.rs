use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Context, OutputChecker, Task};
use crate::trait_object::ValueEqObj;

/// [Task output checker](OutputChecker) that checks by equality.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct EqualsChecker;
impl OutputChecker for EqualsChecker {
  #[inline]
  fn stamp(&self, output: &dyn ValueEqObj) -> Box<dyn ValueEqObj> {
    output.to_owned()
  }

  #[inline]
  fn check<'i>(&self, output: &'i dyn ValueEqObj, stamp: &'i dyn ValueEqObj) -> Option<Box<dyn Debug + 'i>> {
    if output != stamp {
      Some(Box::new(output))
    } else {
      None
    }
  }
}

// /// [Task output checker](OutputChecker) that checks [Ok] by equality, but [Err] only by existence.
// #[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
// pub struct OkEqualsChecker;
// impl<T: Value + Eq, E> OutputChecker<Result<T, E>> for OkEqualsChecker {
//   #[inline]
//   fn stamp(&self, output: &dyn ValueObj) -> Box<dyn ValueObj> {
//     output.as_any().downcast_ref()
//     output.as_ref().ok().cloned()
//   }
//
//   #[inline]
//   fn check(&self, output: &dyn ValueObj, stamp: &dyn ValueObj) -> Option<Box<dyn Debug>> {
//     let new_stamp = output.as_ref().ok();
//     if new_stamp != stamp.as_ref() {
//       Some(new_stamp)
//     } else {
//       None
//     }
//   }
// }
//
// /// [Task output checker](OutputChecker) that checks [Err] by equality, but [Ok] only by existence.
// #[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
// pub struct ErrEqualsChecker;
// impl<T, E: Value + Eq> OutputChecker<Result<T, E>> for ErrEqualsChecker {
//   #[inline]
//   fn stamp(&self, output: &dyn ValueObj) -> Box<dyn ValueObj> {
//     output.as_ref().err().cloned()
//   }
//
//   #[inline]
//   fn check(&self, output: &dyn ValueObj, stamp: &dyn ValueObj) -> Option<Box<dyn Debug>> {
//     let new_stamp = output.as_ref().err();
//     if new_stamp != stamp.as_ref() {
//       Some(new_stamp)
//     } else {
//       None
//     }
//   }
// }
//
// /// [Task output checker](OutputChecker) that checks whether a [Result] changes from [Ok] to [Err] or vice versa.
// #[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
// pub struct ResultChecker;
// impl<T, E> OutputChecker<Result<T, E>> for ResultChecker {
//   #[inline]
//   fn stamp(&self, output: &dyn ValueObj) -> Box<dyn ValueObj> {
//     output.is_err()
//   }
//
//   #[inline]
//   fn check(&self, output: &dyn ValueObj, stamp: &dyn ValueObj) -> Option<Box<dyn Debug>> {
//     let new_stamp = output.is_err();
//     if new_stamp != *stamp {
//       Some(new_stamp)
//     } else {
//       None
//     }
//   }
// }

/// [Task output checker](OutputChecker) that marks task dependencies as always consistent. Can be used to ignore task
/// outputs. For example, this is useful when depending on a task to write to some file which you want to read, but you
/// are not interested in the output of the task.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AlwaysConsistent;
impl OutputChecker for AlwaysConsistent {
  #[inline]
  fn stamp(&self, _output: &dyn ValueEqObj) -> Box<dyn ValueEqObj> {
    Box::new(())
  }

  #[inline]
  fn check<'i>(&self, _output: &'i dyn ValueEqObj, _stamp: &'i dyn ValueEqObj) -> Option<Box<dyn Debug + 'i>> {
    None::<Box<dyn Debug>>
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
