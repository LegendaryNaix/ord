#![allow(clippy::type_complexity)]

use {
  self::{command_builder::CommandBuilder, expected::Expected, test_server::TestServer},
  bip39::Mnemonic,
  bitcoin::{
    address::{Address, NetworkUnchecked},
    blockdata::constants::COIN_VALUE,
    Network, OutPoint, Txid,
  },
  executable_path::executable_path,
  ord::inscription_id::InscriptionId,
  ord::rarity::Rarity,
  ord::subcommand::wallet::send::Output,
  ord::templates::inscription::InscriptionJson,
  ord::templates::inscriptions::InscriptionsJson,
  ord::templates::output::OutputJson,
  ord::templates::sat::SatJson,
  ord::SatPoint,
  pretty_assertions::assert_eq as pretty_assert_eq,
  regex::Regex,
  reqwest::{StatusCode, Url},
  serde::{de::DeserializeOwned, Deserialize},
  std::{
    fs,
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    str::{self, FromStr},
    thread,
    time::Duration,
  },
  tempfile::TempDir,
  test_bitcoincore_rpc::Sent,
  test_bitcoincore_rpc::TransactionTemplate,
};

macro_rules! assert_regex_match {
  ($string:expr, $pattern:expr $(,)?) => {
    let regex = Regex::new(&format!("^(?s){}$", $pattern)).unwrap();
    let string = $string;

    if !regex.is_match(string.as_ref()) {
      panic!(
        "Regex:\n\n{}\n\n…did not match string:\n\n{}",
        regex, string
      );
    }
  };
}

#[derive(Deserialize, Debug)]
struct Inscribe {
  #[allow(dead_code)]
  commit: Txid,
  inscription: String,
  reveal: Txid,
  fees: u64,
}

fn inscribe(rpc_server: &test_bitcoincore_rpc::Handle) -> Inscribe {
  rpc_server.mine_blocks(1);

  let output = CommandBuilder::new("wallet inscribe --fee-rate 1 foo.txt")
    .write("foo.txt", "FOO")
    .rpc_server(rpc_server)
    .run_and_check_output();

  rpc_server.mine_blocks(1);

  output
}

fn envelope(payload: &[&[u8]]) -> bitcoin::Witness {
  let mut builder = bitcoin::script::Builder::new()
    .push_opcode(bitcoin::opcodes::OP_FALSE)
    .push_opcode(bitcoin::opcodes::all::OP_IF);

  for data in payload {
    let mut buf = bitcoin::script::PushBytesBuf::new();
    buf.extend_from_slice(data).unwrap();
    builder = builder.push_slice(buf);
  }

  let script = builder
    .push_opcode(bitcoin::opcodes::all::OP_ENDIF)
    .into_script();

  bitcoin::Witness::from_slice(&[script.into_bytes(), Vec::new()])
}

#[derive(Deserialize)]
struct Create {
  mnemonic: Mnemonic,
}

fn create_wallet(rpc_server: &test_bitcoincore_rpc::Handle) {
  CommandBuilder::new(format!("--chain {} wallet create", rpc_server.network()))
    .rpc_server(rpc_server)
    .run_and_check_output::<Create>();
}

mod command_builder;
mod expected;
mod test_server;

mod core;
mod epochs;
mod find;
mod index;
mod info;
mod json_api;
mod list;
mod parse;
mod server;
mod subsidy;
mod supply;
mod traits;
mod version;
mod wallet;
