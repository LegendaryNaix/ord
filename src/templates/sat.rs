use super::*;

#[derive(Boilerplate)]
pub(crate) struct SatHtml {
  pub(crate) sat: Sat,
  pub(crate) satpoint: Option<SatPoint>,
  pub(crate) blocktime: Blocktime,
  pub(crate) inscription: Option<InscriptionId>,
}

impl PageContent for SatHtml {
  fn title(&self) -> String {
    format!("Sat {}", self.sat)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn first() {
    assert_regex_match!(
      SatHtml {
        sat: Sat(0),
        satpoint: None,
        blocktime: Blocktime::confirmed(0),
        inscription: None,
      },
      "
        <h1>Sat 0</h1>
        <dl>
          <dt>decimal</dt><dd>0.0</dd>
          <dt>degree</dt><dd>0°0′0″0‴</dd>
          <dt>percentile</dt><dd>0%</dd>
          <dt>name</dt><dd>nvtdijuwxlp</dd>
          <dt>cycle</dt><dd>0</dd>
          <dt>epoch</dt><dd>0</dd>
          <dt>period</dt><dd>0</dd>
          <dt>block</dt><dd><a href=/block/0>0</a></dd>
          <dt>offset</dt><dd>0</dd>
          <dt>rarity</dt><dd><span class=mythic>mythic</span></dd>
          <dt>timestamp</dt><dd><time>1970-01-01 00:00:00 UTC</time></dd>
        </dl>
        .*
        prev
        <a class=next href=/sat/1>next</a>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn last() {
    assert_regex_match!(
      SatHtml {
        sat: Sat(2099999997689999),
        satpoint: None,
        blocktime: Blocktime::confirmed(0),
        inscription: None,
      },
      "
        <h1>Sat 2099999997689999</h1>
        <dl>
          <dt>decimal</dt><dd>6929999.0</dd>
          <dt>degree</dt><dd>5°209999′1007″0‴</dd>
          <dt>percentile</dt><dd>100%</dd>
          <dt>name</dt><dd>a</dd>
          <dt>cycle</dt><dd>5</dd>
          <dt>epoch</dt><dd>32</dd>
          <dt>period</dt><dd>3437</dd>
          <dt>block</dt><dd><a href=/block/6929999>6929999</a></dd>
          <dt>offset</dt><dd>0</dd>
          <dt>rarity</dt><dd><span class=uncommon>uncommon</span></dd>
          <dt>timestamp</dt><dd><time>1970-01-01 00:00:00 UTC</time></dd>
        </dl>
        .*
        <a class=prev href=/sat/2099999997689998>prev</a>
        next
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn sat_with_next_and_prev() {
    assert_regex_match!(
      SatHtml {
        sat: Sat(1),
        satpoint: None,
        blocktime: Blocktime::confirmed(0),
        inscription: None,
      },
      r"<h1>Sat 1</h1>.*<a class=prev href=/sat/0>prev</a>\n<a class=next href=/sat/2>next</a>.*",
    );
  }

  #[test]
  fn sat_with_inscription() {
    assert_regex_match!(
      SatHtml {
        sat: Sat(0),
        satpoint: None,
        blocktime: Blocktime::confirmed(0),
        inscription: Some(inscription_id(1)),
      },
      r"<h1>Sat 0</h1>.*<dt>inscription</dt><dd class=thumbnails><a href=/inscription/1{64}i1>.*</a></dd>.*",
    );
  }

  #[test]
  fn last_sat_next_link_is_disabled() {
    assert_regex_match!(
      SatHtml {
        sat: Sat::LAST,
        satpoint: None,
        blocktime: Blocktime::confirmed(0),
        inscription: None,
      },
      r"<h1>Sat 2099999997689999</h1>.*<a class=prev href=/sat/2099999997689998>prev</a>\nnext.*",
    );
  }

  #[test]
  fn sat_with_satpoint() {
    assert_regex_match!(
      SatHtml {
        sat: Sat(0),
        satpoint: Some(satpoint(1, 0)),
        blocktime: Blocktime::confirmed(0),
        inscription: None,
      },
      "<h1>Sat 0</h1>.*<dt>location</dt><dd class=monospace>1{64}:1:0</dd>.*",
    );
  }
}
