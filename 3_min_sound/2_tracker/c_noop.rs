
/// [`Tracker`] that does nothing.
pub struct NoopTracker;
impl<T: Task> Tracker<T> for NoopTracker {}
