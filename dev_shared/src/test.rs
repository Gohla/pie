use pie::Task;
use pie::tracker::CompositeTracker;
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;
use std::io::Stdout;

pub type Pie<T> = pie::Pie<T, CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>>;

pub fn create_pie<T: Task>() -> Pie<T> {
  let tracker = CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer());
  Pie::with_tracker(tracker)
}
