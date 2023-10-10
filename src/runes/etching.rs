use super::*;

#[derive(Default, Serialize, Debug, PartialEq, Copy, Clone)]
pub struct Etching {
  pub(crate) divisibility: u8,
  pub(crate) rune: Rune,
}
