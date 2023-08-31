use {
  super::*,
  bitcoin::{
    blockdata::{
      opcodes,
      script::{self, Instruction, Instructions, PushBytesBuf},
    },
    ScriptBuf, Witness,
  },
  std::{iter::Peekable, str},
};

const PROTOCOL_ID: [u8; 3] = *b"ord";
const BODY_TAG: [u8; 0] = [];
const CONTENT_TYPE_TAG: [u8; 1] = [1];

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Curse {
  NotInFirstInput,
  NotAtOffsetZero,
  Reinscription,
  UnrecognizedEvenField,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Inscription {
  body: Option<Vec<u8>>,
  content_type: Option<Vec<u8>>,
  pub(crate) unrecognized_even_field: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TransactionInscription {
  pub(crate) inscription: Inscription,
  pub(crate) tx_in_index: u32,
  pub(crate) tx_in_offset: u32,
}

impl Inscription {
  #[cfg(test)]
  pub(crate) fn new(content_type: Option<Vec<u8>>, body: Option<Vec<u8>>) -> Self {
    Self {
      content_type,
      body,
      unrecognized_even_field: false,
    }
  }

  pub(crate) fn from_transaction(tx: &Transaction) -> Vec<TransactionInscription> {
    let mut result = Vec::new();
    for (index, tx_in) in tx.input.iter().enumerate() {
      let Ok(inscriptions) = InscriptionParser::parse(&tx_in.witness) else {
        continue;
      };

      result.extend(
        inscriptions
          .into_iter()
          .enumerate()
          .map(|(offset, inscription)| TransactionInscription {
            inscription,
            tx_in_index: u32::try_from(index).unwrap(),
            tx_in_offset: u32::try_from(offset).unwrap(),
          })
          .collect::<Vec<TransactionInscription>>(),
      )
    }

    result
  }

  pub(crate) fn from_file(chain: Chain, path: impl AsRef<Path>) -> Result<Self, Error> {
    let path = path.as_ref();

    let body = fs::read(path).with_context(|| format!("io error reading {}", path.display()))?;

    if let Some(limit) = chain.inscription_content_size_limit() {
      let len = body.len();
      if len > limit {
        bail!("content size of {len} bytes exceeds {limit} byte limit for {chain} inscriptions");
      }
    }

    let content_type = Media::content_type_for_path(path)?;

    Ok(Self {
      body: Some(body),
      content_type: Some(content_type.into()),
      unrecognized_even_field: false,
    })
  }

  fn append_reveal_script_to_builder(&self, mut builder: script::Builder) -> script::Builder {
    builder = builder
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(PROTOCOL_ID);

    if let Some(content_type) = self.content_type.clone() {
      builder = builder
        .push_slice(CONTENT_TYPE_TAG)
        .push_slice(PushBytesBuf::try_from(content_type).unwrap());
    }

    if let Some(body) = &self.body {
      builder = builder.push_slice(BODY_TAG);
      for chunk in body.chunks(520) {
        builder = builder.push_slice(PushBytesBuf::try_from(chunk.to_vec()).unwrap());
      }
    }

    builder.push_opcode(opcodes::all::OP_ENDIF)
  }

  pub(crate) fn append_reveal_script(&self, builder: script::Builder) -> ScriptBuf {
    self.append_reveal_script_to_builder(builder).into_script()
  }

  pub(crate) fn media(&self) -> Media {
    if self.body.is_none() {
      return Media::Unknown;
    }

    let Some(content_type) = self.content_type() else {
      return Media::Unknown;
    };

    content_type.parse().unwrap_or(Media::Unknown)
  }

  pub(crate) fn body(&self) -> Option<&[u8]> {
    Some(self.body.as_ref()?)
  }

  pub(crate) fn into_body(self) -> Option<Vec<u8>> {
    self.body
  }

  pub(crate) fn content_length(&self) -> Option<usize> {
    Some(self.body()?.len())
  }

  pub(crate) fn content_type(&self) -> Option<&str> {
    str::from_utf8(self.content_type.as_ref()?).ok()
  }

  #[cfg(test)]
  pub(crate) fn to_witness(&self) -> Witness {
    let builder = script::Builder::new();

    let script = self.append_reveal_script(builder);

    let mut witness = Witness::new();

    witness.push(script);
    witness.push([]);

    witness
  }
}

#[derive(Debug, PartialEq)]
pub(crate) enum InscriptionError {
  InvalidInscription,
  NoInscription,
  NoTapscript,
  Script(script::Error),
}

type Result<T, E = InscriptionError> = std::result::Result<T, E>;

#[derive(Debug)]
struct InscriptionParser<'a> {
  instructions: Peekable<Instructions<'a>>,
}

impl<'a> InscriptionParser<'a> {
  fn parse(witness: &Witness) -> Result<Vec<Inscription>> {
    let Some(tapscript) = witness.tapscript() else {
      return Err(InscriptionError::NoTapscript);
    };

    InscriptionParser {
      instructions: tapscript.instructions().peekable(),
    }
    .parse_inscriptions()
    .into_iter()
    .collect()
  }

  fn parse_inscriptions(&mut self) -> Vec<Result<Inscription>> {
    let mut inscriptions = Vec::new();
    loop {
      let current = self.parse_one_inscription();
      if current == Err(InscriptionError::NoInscription) {
        break;
      }
      inscriptions.push(current);
    }

    inscriptions
  }

  fn parse_one_inscription(&mut self) -> Result<Inscription> {
    self.advance_into_inscription_envelope()?;
    let mut fields = BTreeMap::new();

    loop {
      match self.advance()? {
        Instruction::PushBytes(tag) if tag.as_bytes() == BODY_TAG.as_slice() => {
          let mut body = Vec::new();
          while !self.accept(&Instruction::Op(opcodes::all::OP_ENDIF))? {
            body.extend_from_slice(self.expect_push()?);
          }
          fields.insert(BODY_TAG.as_slice(), body);
          break;
        }
        Instruction::PushBytes(tag) => {
          if fields.contains_key(tag.as_bytes()) {
            return Err(InscriptionError::InvalidInscription);
          }
          fields.insert(tag.as_bytes(), self.expect_push()?.to_vec());
        }
        Instruction::Op(opcodes::all::OP_ENDIF) => break,
        _ => return Err(InscriptionError::InvalidInscription),
      }
    }

    let body = fields.remove(BODY_TAG.as_slice());
    let content_type = fields.remove(CONTENT_TYPE_TAG.as_slice());

    for tag in fields.keys() {
      if let Some(lsb) = tag.first() {
        if lsb % 2 == 0 {
          return Ok(Inscription {
            body,
            content_type,
            unrecognized_even_field: true,
          });
        }
      }
    }

    Ok(Inscription {
      body,
      content_type,
      unrecognized_even_field: false,
    })
  }

  fn advance(&mut self) -> Result<Instruction<'a>> {
    self
      .instructions
      .next()
      .ok_or(InscriptionError::NoInscription)?
      .map_err(InscriptionError::Script)
  }

  fn advance_into_inscription_envelope(&mut self) -> Result<()> {
    loop {
      if self.match_instructions(&[
        Instruction::PushBytes((&[]).into()), // represents an OF_FALSE
        Instruction::Op(opcodes::all::OP_IF),
        Instruction::PushBytes((&PROTOCOL_ID).into()),
      ])? {
        break;
      }
    }

    Ok(())
  }

  fn match_instructions(&mut self, instructions: &[Instruction]) -> Result<bool> {
    for instruction in instructions {
      if &self.advance()? != instruction {
        return Ok(false);
      }
    }

    Ok(true)
  }

  fn expect_push(&mut self) -> Result<&'a [u8]> {
    match self.advance()? {
      Instruction::PushBytes(bytes) => Ok(bytes.as_bytes()),
      _ => Err(InscriptionError::InvalidInscription),
    }
  }

  fn accept(&mut self, instruction: &Instruction) -> Result<bool> {
    match self.instructions.peek() {
      Some(Ok(next)) => {
        if next == instruction {
          self.advance()?;
          Ok(true)
        } else {
          Ok(false)
        }
      }
      Some(Err(err)) => Err(InscriptionError::Script(*err)),
      None => Ok(false),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty() {
    assert_eq!(
      InscriptionParser::parse(&Witness::new()),
      Err(InscriptionError::NoTapscript)
    );
  }

  #[test]
  fn ignore_key_path_spends() {
    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[Vec::new()])),
      Err(InscriptionError::NoTapscript),
    );
  }

  #[test]
  fn ignore_key_path_spends_with_annex() {
    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[Vec::new(), vec![0x50]])),
      Err(InscriptionError::NoTapscript),
    );
  }

  #[test]
  fn ignore_unparsable_scripts() {
    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[vec![0x01], Vec::new()])),
      Err(InscriptionError::Script(script::Error::EarlyEndOfScript)),
    );
  }

  #[test]
  fn no_inscription() {
    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[
        ScriptBuf::new().into_bytes(),
        Vec::new()
      ])),
      Ok(vec![])
    );
  }

  #[test]
  fn duplicate_field() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[1],
        b"text/plain;charset=utf-8",
        &[],
        b"ord",
      ])),
      Err(InscriptionError::InvalidInscription),
    );
  }

  #[test]
  fn valid() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[],
        b"ord",
      ])),
      Ok(vec![inscription("text/plain;charset=utf-8", "ord")]),
    );
  }

  #[test]
  fn valid_with_unknown_tag() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[3],
        b"bar",
        &[],
        b"ord",
      ])),
      Ok(vec![inscription("text/plain;charset=utf-8", "ord")]),
    );
  }

  #[test]
  fn no_content_tag() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[1], b"text/plain;charset=utf-8"])),
      Ok(vec![Inscription {
        content_type: Some(b"text/plain;charset=utf-8".to_vec()),
        body: None,
        unrecognized_even_field: false,
      }]),
    );
  }

  #[test]
  fn no_content_type() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[], b"foo"])),
      Ok(vec![Inscription {
        content_type: None,
        body: Some(b"foo".to_vec()),
        unrecognized_even_field: false,
      }]),
    );
  }

  #[test]
  fn valid_body_in_multiple_pushes() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[],
        b"foo",
        b"bar"
      ])),
      Ok(vec![inscription("text/plain;charset=utf-8", "foobar")]),
    );
  }

  #[test]
  fn valid_body_in_zero_pushes() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[1], b"text/plain;charset=utf-8", &[]])),
      Ok(vec![inscription("text/plain;charset=utf-8", "")]),
    );
  }

  #[test]
  fn valid_body_in_multiple_empty_pushes() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
      ])),
      Ok(vec![inscription("text/plain;charset=utf-8", "")]),
    );
  }

  #[test]
  fn valid_ignore_trailing() {
    let script = script::Builder::new()
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .push_slice([1])
      .push_slice(b"text/plain;charset=utf-8")
      .push_slice([])
      .push_slice(b"ord")
      .push_opcode(opcodes::all::OP_ENDIF)
      .push_opcode(opcodes::all::OP_CHECKSIG)
      .into_script();

    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[script.into_bytes(), Vec::new()])),
      Ok(vec![inscription("text/plain;charset=utf-8", "ord")]),
    );
  }

  #[test]
  fn valid_ignore_preceding() {
    let script = script::Builder::new()
      .push_opcode(opcodes::all::OP_CHECKSIG)
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .push_slice([1])
      .push_slice(b"text/plain;charset=utf-8")
      .push_slice([])
      .push_slice(b"ord")
      .push_opcode(opcodes::all::OP_ENDIF)
      .into_script();

    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[script.into_bytes(), Vec::new()])),
      Ok(vec![inscription("text/plain;charset=utf-8", "ord")]),
    );
  }

  #[test]
  fn do_not_ignore_inscriptions_after_first() {
    let script = script::Builder::new()
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .push_slice([1])
      .push_slice(b"text/plain;charset=utf-8")
      .push_slice([])
      .push_slice(b"foo")
      .push_opcode(opcodes::all::OP_ENDIF)
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .push_slice([1])
      .push_slice(b"text/plain;charset=utf-8")
      .push_slice([])
      .push_slice(b"bar")
      .push_opcode(opcodes::all::OP_ENDIF)
      .into_script();

    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[script.into_bytes(), Vec::new()])),
      Ok(vec![
        inscription("text/plain;charset=utf-8", "foo"),
        inscription("text/plain;charset=utf-8", "bar")
      ]),
    );
  }

  #[test]
  fn invalid_utf8_does_not_render_inscription_invalid() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[
        b"ord",
        &[1],
        b"text/plain;charset=utf-8",
        &[],
        &[0b10000000]
      ])),
      Ok(vec![inscription("text/plain;charset=utf-8", [0b10000000])]),
    );
  }

  #[test]
  fn no_endif() {
    let script = script::Builder::new()
      .push_opcode(opcodes::OP_FALSE)
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .into_script();

    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[script.into_bytes(), Vec::new()])),
      Ok(vec![])
    );
  }

  #[test]
  fn no_op_false() {
    let script = script::Builder::new()
      .push_opcode(opcodes::all::OP_IF)
      .push_slice(b"ord")
      .push_opcode(opcodes::all::OP_ENDIF)
      .into_script();

    assert_eq!(
      InscriptionParser::parse(&Witness::from_slice(&[script.into_bytes(), Vec::new()])),
      Ok(vec![])
    );
  }

  #[test]
  fn empty_envelope() {
    assert_eq!(InscriptionParser::parse(&envelope(&[])), Ok(vec![]));
  }

  #[test]
  fn wrong_magic_number() {
    assert_eq!(InscriptionParser::parse(&envelope(&[b"foo"])), Ok(vec![]));
  }

  #[test]
  fn extract_from_transaction() {
    let tx = Transaction {
      version: 0,
      lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
      input: vec![TxIn {
        previous_output: OutPoint::null(),
        script_sig: ScriptBuf::new(),
        sequence: Sequence(0),
        witness: envelope(&[b"ord", &[1], b"text/plain;charset=utf-8", &[], b"ord"]),
      }],
      output: Vec::new(),
    };

    assert_eq!(
      Inscription::from_transaction(&tx),
      vec![transaction_inscription(
        "text/plain;charset=utf-8",
        "ord",
        0,
        0
      )],
    );
  }

  #[test]
  fn extract_from_second_input() {
    let tx = Transaction {
      version: 0,
      lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
      input: vec![
        TxIn {
          previous_output: OutPoint::null(),
          script_sig: ScriptBuf::new(),
          sequence: Sequence(0),
          witness: Witness::new(),
        },
        TxIn {
          previous_output: OutPoint::null(),
          script_sig: ScriptBuf::new(),
          sequence: Sequence(0),
          witness: inscription("foo", [1; 1040]).to_witness(),
        },
      ],
      output: Vec::new(),
    };

    assert_eq!(
      Inscription::from_transaction(&tx),
      vec![transaction_inscription("foo", [1; 1040], 1, 0)]
    );
  }

  #[test]
  fn extract_from_second_envelope() {
    let mut builder = script::Builder::new();
    builder = inscription("foo", [1; 100]).append_reveal_script_to_builder(builder);
    builder = inscription("bar", [1; 100]).append_reveal_script_to_builder(builder);

    let witness = Witness::from_slice(&[builder.into_script().into_bytes(), Vec::new()]);

    let tx = Transaction {
      version: 0,
      lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
      input: vec![TxIn {
        previous_output: OutPoint::null(),
        script_sig: ScriptBuf::new(),
        sequence: Sequence(0),
        witness,
      }],
      output: Vec::new(),
    };

    assert_eq!(
      Inscription::from_transaction(&tx),
      vec![
        transaction_inscription("foo", [1; 100], 0, 0),
        transaction_inscription("bar", [1; 100], 0, 1)
      ]
    );
  }

  #[test]
  fn inscribe_png() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[1], b"image/png", &[], &[1; 100]])),
      Ok(vec![inscription("image/png", [1; 100])]),
    );
  }

  #[test]
  fn reveal_script_chunks_data() {
    assert_eq!(
      inscription("foo", [])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      7
    );

    assert_eq!(
      inscription("foo", [0; 1])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      8
    );

    assert_eq!(
      inscription("foo", [0; 520])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      8
    );

    assert_eq!(
      inscription("foo", [0; 521])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      9
    );

    assert_eq!(
      inscription("foo", [0; 1040])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      9
    );

    assert_eq!(
      inscription("foo", [0; 1041])
        .append_reveal_script(script::Builder::new())
        .instructions()
        .count(),
      10
    );
  }

  #[test]
  fn chunked_data_is_parsable() {
    let mut witness = Witness::new();

    witness.push(&inscription("foo", [1; 1040]).append_reveal_script(script::Builder::new()));

    witness.push([]);

    assert_eq!(
      InscriptionParser::parse(&witness).unwrap(),
      vec![inscription("foo", [1; 1040])],
    );
  }

  #[test]
  fn round_trip_with_no_fields() {
    let mut witness = Witness::new();

    witness.push(
      &Inscription {
        content_type: None,
        body: None,
        unrecognized_even_field: false,
      }
      .append_reveal_script(script::Builder::new()),
    );

    witness.push([]);

    assert_eq!(
      InscriptionParser::parse(&witness).unwrap(),
      vec![Inscription {
        content_type: None,
        body: None,
        unrecognized_even_field: false,
      }]
    );
  }

  #[test]
  fn unknown_odd_fields_are_ignored() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[3], &[0]])),
      Ok(vec![Inscription {
        content_type: None,
        body: None,
        unrecognized_even_field: false,
      }]),
    );
  }

  #[test]
  fn unknown_even_fields() {
    assert_eq!(
      InscriptionParser::parse(&envelope(&[b"ord", &[2], &[0]])),
      Ok(vec![Inscription {
        content_type: None,
        body: None,
        unrecognized_even_field: true,
      }]),
    );
  }
}
