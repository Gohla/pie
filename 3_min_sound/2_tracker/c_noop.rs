
/// [`Tracker`] that does nothing.
#[derive(Copy, Clone, Debug)]
pub struct NoopTracker;
impl<T: Task> Tracker<T> for NoopTracker {}
