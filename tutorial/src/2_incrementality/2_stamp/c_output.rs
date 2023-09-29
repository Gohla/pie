

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum OutputStamper {
  Inconsequential,
  Equals,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum OutputStamp<O> {
  Inconsequential,
  Equals(O),
}

impl OutputStamper {
  pub fn stamp<O>(&self, output: O) -> OutputStamp<O> {
    match self {
      OutputStamper::Inconsequential => OutputStamp::Inconsequential,
      OutputStamper::Equals => OutputStamp::Equals(output),
    }
  }
}
