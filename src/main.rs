#![allow(clippy::too_many_arguments)]

use {
  self::{
    arguments::Arguments,
    blocktime::Blocktime,
    bytes::Bytes,
    degree::Degree,
    epoch::Epoch,
    height::Height,
    index::{Index, List},
    nft::Nft,
    options::Options,
    ordinal::Ordinal,
    purse::Purse,
    rarity::Rarity,
    sat_point::SatPoint,
    subcommand::Subcommand,
  },
  anyhow::{anyhow, bail, Context, Error},
  axum::{extract, http::StatusCode, response::Html, response::IntoResponse, routing::get, Router},
  axum_server::Handle,
  bdk::{
    blockchain::rpc::{Auth, RpcBlockchain, RpcConfig},
    blockchain::{Blockchain, ConfigurableBlockchain},
    database::SqliteDatabase,
    keys::bip39::{Language, Mnemonic},
    template::Bip84,
    wallet::{signer::SignOptions, AddressIndex::LastUnused},
    wallet::{wallet_name_from_descriptor, SyncOptions},
    FeeRate, KeychainKind, LocalUtxo,
  },
  bitcoin::{
    blockdata::{constants::COIN_VALUE, transaction::TxOut},
    consensus::{Decodable, Encodable},
    hash_types::BlockHash,
    hashes::{sha256, sha256d, Hash, HashEngine},
    secp256k1::{
      self,
      rand::{self, thread_rng},
      schnorr::Signature,
      KeyPair, Secp256k1, XOnlyPublicKey,
    },
    util::{key::PrivateKey, psbt::PartiallySignedTransaction},
    Address, Block, Network, OutPoint, Transaction, Txid,
  },
  chrono::{DateTime, NaiveDateTime, Utc},
  clap::Parser,
  derive_more::{Display, FromStr},
  redb::{Database, ReadableTable, Table, TableDefinition, WriteTransaction},
  serde::{Deserialize, Serialize},
  std::{
    cmp::Ordering,
    collections::VecDeque,
    env,
    fmt::{self, Display, Formatter},
    fs,
    io::{self, Write},
    net::ToSocketAddrs,
    ops::{Add, AddAssign, Deref, Sub},
    path::{Path, PathBuf},
    process,
    str::FromStr,
    sync::{
      atomic::{self, AtomicU64},
      Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
  },
  tokio::{runtime::Runtime, task},
  tower_http::cors::{Any, CorsLayer},
};

#[cfg(test)]
use regex::Regex;

#[cfg(test)]
macro_rules! assert_regex_match {
  ($string:expr, $pattern:expr $(,)?) => {
    let pattern: &'static str = $pattern;
    let regex = Regex::new(&format!("^(?s){}$", pattern)).unwrap();
    let string = $string;

    if !regex.is_match(string.as_ref()) {
      panic!(
        "Regex:\n\n{}\n\n…did not match string:\n\n{}",
        regex, string
      );
    }
  };
}

mod arguments;
mod blocktime;
mod bytes;
mod degree;
mod epoch;
mod height;
mod index;
mod nft;
mod options;
mod ordinal;
mod purse;
mod rarity;
mod sat_point;
mod subcommand;

type Result<T = (), E = Error> = std::result::Result<T, E>;

const PERIOD_BLOCKS: u64 = 2016;
const CYCLE_EPOCHS: u64 = 6;

static INTERRUPTS: AtomicU64 = AtomicU64::new(0);
static LISTENERS: Mutex<Vec<Handle>> = Mutex::new(Vec::new());

fn main() {
  env_logger::init();

  ctrlc::set_handler(move || {
    LISTENERS
      .lock()
      .unwrap()
      .iter()
      .for_each(|handle| handle.graceful_shutdown(Some(Duration::from_millis(100))));

    let interrupts = INTERRUPTS.fetch_add(1, atomic::Ordering::Relaxed);

    if interrupts > 5 {
      process::exit(1);
    }
  })
  .expect("Error setting ctrl-c handler");

  if let Err(err) = Arguments::parse().run() {
    eprintln!("error: {}", err);
    err
      .chain()
      .skip(1)
      .for_each(|cause| eprintln!("because: {}", cause));
    if env::var_os("RUST_BACKTRACE")
      .map(|val| val == "1")
      .unwrap_or_default()
    {
      eprintln!("{}", err.backtrace());
    }
    process::exit(1);
  }
}
