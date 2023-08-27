use super::*;

#[derive(Boilerplate)]
pub(crate) struct InscriptionsBlockHtml {
  pub(crate) block: u64,
  pub(crate) inscriptions: Vec<InscriptionId>,
  pub(crate) prev_block: Option<u64>,
  pub(crate) next_block: Option<u64>,
  pub(crate) prev_page: Option<usize>,
  pub(crate) next_page: Option<usize>,
}

impl InscriptionsBlockHtml {
  pub(crate) fn new(
    block: u64,
    current_blockheight: u64,
    inscriptions: Vec<InscriptionId>,
    page_index: usize,
  ) -> Result<Self> {
    let start = page_index * 100;
    let end = start + 100;
    let num_inscriptions = inscriptions.len();
    let inscriptions = inscriptions[start..end].to_vec();

    Ok(Self {
      block,
      inscriptions,
      prev_block: if block == 0 { None } else { Some(block - 1) },
      next_block: if block >= current_blockheight {
        None
      } else {
        Some(block + 1)
      },
      prev_page: if page_index > 0 {
        Some(page_index - 1)
      } else {
        None
      },
      next_page: if page_index * 100 <= num_inscriptions {
        Some(page_index + 1)
      } else {
        None
      },
    })
  }
}

impl PageContent for InscriptionsBlockHtml {
  fn title(&self) -> String {
    format!("Inscriptions in Block {0}", self.block)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn without_prev_and_next() {
    assert_regex_match!(
      InscriptionsBlockHtml {
        block: 21,
        inscriptions: vec![inscription_id(1), inscription_id(2)],
        prev_block: None,
        next_block: None,
        prev_page: None,
        next_page: None,
      },
      "
        <h1>Inscriptions in <a href=/block/21>Block 21</a></h1>
        <div class=thumbnails>
          <a href=/inscription/1{64}i1><iframe .* src=/preview/1{64}i1></iframe></a>
          <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2></iframe></a>
        </div>
        .*
        prev
        next
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn with_prev_and_next() {
    assert_regex_match!(
      InscriptionsBlockHtml {
        block: 21,
        inscriptions: vec![inscription_id(1), inscription_id(2)],
        prev_block: Some(20),
        next_block: Some(22),
        next_page: Some(3),
        prev_page: Some(1),
      },
      "
        <h1>Inscriptions in <a href=/block/21>Block 21</a></h1>
        <div class=thumbnails>
          <a href=/inscription/1{64}i1><iframe .* src=/preview/1{64}i1></iframe></a>
          <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2></iframe></a>
        </div>
        .*
          <a class=prev href=/inscriptions/block/20>20</a>
        &bull;
          <a class=prev href=/inscriptions/block/21/1>prev</a>
          <a class=next href=/inscriptions/block/21/3>next</a>
        &bull;
          <a class=next href=/inscriptions/block/22>22</a>
        .*
      "
      .unindent()
    );
  }
}
