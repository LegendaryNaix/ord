use super::*;

#[derive(Boilerplate)]
pub(crate) struct HomeHtml {
  last: u64,
  blocks: Vec<BlockHash>,
  inscriptions: Vec<InscriptionId>,
}

impl HomeHtml {
  pub(crate) fn new(blocks: Vec<(u64, BlockHash)>, inscriptions: Vec<InscriptionId>) -> Self {
    Self {
      last: blocks
        .get(0)
        .map(|(height, _)| height)
        .cloned()
        .unwrap_or(0),
      blocks: blocks.into_iter().map(|(_, hash)| hash).collect(),
      inscriptions,
    }
  }
}

impl PageContent for HomeHtml {
  fn title(&self) -> String {
    "Ordinals".to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn html() {
    assert_regex_match!(
      &HomeHtml::new(
        vec![
          (
            1260001,
            "1111111111111111111111111111111111111111111111111111111111111111"
              .parse()
              .unwrap()
          ),
          (
            1260000,
            "0000000000000000000000000000000000000000000000000000000000000000"
              .parse()
              .unwrap()
          )
        ],
        vec![inscription_id(1), inscription_id(2)],
      )
      .to_string(),
      "<h2>Latest Inscriptions</h2>
<div class=thumbnails>
  <a href=/inscription/1{64}i1><iframe .* src=/preview/1{64}i1></iframe></a>
  <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2></iframe></a>
</div>
<div class=center><a href=/inscriptions>more</a></div>
<h2>Latest Blocks</h2>
<ol start=1260001 reversed class=blocks>
  <li><a href=/block/1{64}>1{64}</a></li>
  <li><a href=/block/0{64}>0{64}</a></li>
</ol>
",
    );
  }
}
