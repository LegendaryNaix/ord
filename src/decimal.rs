use super::*;

#[derive(PartialEq, Debug)]
pub(crate) struct Decimal {
  height: Height,
  offset: u64,
}

impl From<Sat> for Decimal {
  fn from(sat: Sat) -> Self {
    Self {
      height: sat.height(),
      offset: sat.third(),
    }
  }
}

impl Display for Decimal {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}.{}", self.height, self.offset)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decimal() {
    assert_eq!(
      Sat(0).decimal(),
      Decimal {
        height: Height(0),
        offset: 0
      }
    );
    assert_eq!(
      Sat(1).decimal(),
      Decimal {
        height: Height(0),
        offset: 1
      }
    );
    assert_eq!(
      Sat(2099999997689999).decimal(),
      Decimal {
        height: Height(6929999),
        offset: 0
      }
    );
  }
}
