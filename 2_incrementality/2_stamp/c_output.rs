#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum OutputStamper {
  Inconsequential,
  Equals,
}

impl OutputStamper {
  pub fn stamp<O>(&self, output: O) -> OutputStamp<O> {
    match self {
      OutputStamper::Inconsequential => OutputStamp::Inconsequential,
      OutputStamper::Equals => OutputStamp::Equals(output),
    }
  }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum OutputStamp<O> {
  Inconsequential,
  Equals(O),
}

impl<O> OutputStamp<O> {
  pub fn as_ref(&self) -> OutputStamp<&O> {
    match self {
      OutputStamp::Inconsequential => OutputStamp::Inconsequential,
      OutputStamp::Equals(o) => OutputStamp::Equals(o),
    }
  }
}

