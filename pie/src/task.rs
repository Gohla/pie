use serde::{Deserialize, Serialize};

use crate::{Context, register_task, Task};

/// Task that does nothing and returns `()`.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct NoopTask {}

impl Task for NoopTask {
  type Output = ();
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output { () }
}

register_task!(NoopTask);
