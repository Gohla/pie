use pie::{Context, Pie, Task};
use pie::tracker::writing::WritingTracker;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct HelloWorld;

impl Task for HelloWorld {
  type Output = &'static str;
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
    "Hello, World!"
  }
}

fn main() {
  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());
  let output = pie.run_in_session(|mut session| session.require(&HelloWorld));
  println!("{}", output);
}
