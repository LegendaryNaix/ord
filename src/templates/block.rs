use super::*;

fn target_as_block_hash(target: bitcoin::Target) -> BlockHash {
  BlockHash::from_raw_hash(Hash::from_byte_array(target.to_le_bytes()))
}

#[derive(Boilerplate)]
pub(crate) struct BlockHtml {
  hash: BlockHash,
  target: BlockHash,
  best_height: Height,
  block: Block,
  height: Height,
  inscription_count: usize,
  featured_inscriptions: Vec<InscriptionId>,
}

impl BlockHtml {
  pub(crate) fn new(
    block: Block,
    height: Height,
    best_height: Height,
    inscription_count: usize,
    featured_inscriptions: Vec<InscriptionId>,
  ) -> Self {
    Self {
      hash: block.header.block_hash(),
      target: target_as_block_hash(block.header.target()),
      block,
      height,
      best_height,
      inscription_count,
      featured_inscriptions,
    }
  }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockJson {
  pub hash: BlockHash,
  pub target: BlockHash,
  pub best_height: u64,
  pub height: u64,
  pub inscriptions: Vec<InscriptionId>,
}

impl BlockJson {
  pub(crate) fn new(
    block: Block,
    height: Height,
    best_height: Height,
    inscriptions: Vec<InscriptionId>,
  ) -> Self {
    Self {
      hash: block.header.block_hash(),
      target: target_as_block_hash(block.header.target()),
      height: height.0,
      best_height: best_height.0,
      inscriptions,
    }
  }
}

impl PageContent for BlockHtml {
  fn title(&self) -> String {
    format!("Block {}", self.height)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn html() {
    assert_regex_match!(
      BlockHtml::new(
        Chain::Mainnet.genesis_block(),
        Height(0),
        Height(0),
        0,
        Vec::new()
      ),
      "
        <h1>Block 0</h1>
        <dl>
          <dt>hash</dt><dd class=monospace>[[:xdigit:]]{64}</dd>
          <dt>target</dt><dd class=monospace>[[:xdigit:]]{64}</dd>
          <dt>timestamp</dt><dd><time>2009-01-03 18:15:05 UTC</time></dd>
          <dt>size</dt><dd>285</dd>
          <dt>weight</dt><dd>1140</dd>
        </dl>
        .*
        prev
        next
        .*
        <h2>0 Inscriptions</h2>
        <div class=thumbnails>
        </div>
        <h2>1 Transaction</h2>
        <ul class=monospace>
          <li><a href=/tx/[[:xdigit:]]{64}>[[:xdigit:]]{64}</a></li>
        </ul>
      "
      .unindent()
    );
  }

  #[test]
  fn next_active_when_not_last() {
    assert_regex_match!(
      BlockHtml::new(
        Chain::Mainnet.genesis_block(),
        Height(0),
        Height(1),
        0,
        Vec::new()
      ),
      r"<h1>Block 0</h1>.*prev\s*<a class=next href=/block/1>next</a>.*"
    );
  }

  #[test]
  fn prev_active_when_not_first() {
    assert_regex_match!(
      BlockHtml::new(
        Chain::Mainnet.genesis_block(),
        Height(1),
        Height(1),
        0,
        Vec::new()
      ),
      r"<h1>Block 1</h1>.*<a class=prev href=/block/0>prev</a>\s*next.*",
    );
  }

  #[test]
  fn block_hash_serializes_as_hex_string() {
    assert_eq!(
      serde_json::to_string(&BlockHash::all_zeros()).unwrap(),
      "\"0000000000000000000000000000000000000000000000000000000000000000\""
    );
  }
}
