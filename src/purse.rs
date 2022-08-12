use super::*;

#[derive(Debug)]
pub(crate) struct Purse {
  pub(crate) wallet: bdk::wallet::Wallet<SqliteDatabase>,
  pub(crate) blockchain: RpcBlockchain,
}

impl Purse {
  pub(crate) fn init(options: &Options) -> Result {
    let path = data_dir()
      .ok_or_else(|| anyhow!("Failed to retrieve data dir"))?
      .join("ord");

    if path.exists() {
      return Err(anyhow!("Wallet already exists."));
    }

    fs::create_dir_all(&path)?;

    let seed = Mnemonic::generate_in_with(&mut rand::thread_rng(), Language::English, 12)?;

    fs::write(path.join("entropy"), seed.to_entropy())?;

    let wallet = bdk::wallet::Wallet::new(
      Bip84((seed.clone(), None), KeychainKind::External),
      None,
      options.network,
      SqliteDatabase::new(
        path
          .join("wallet.sqlite")
          .to_str()
          .ok_or_else(|| anyhow!("Failed to convert path to str"))?
          .to_string(),
      ),
    )?;

    wallet.sync(
      &RpcBlockchain::from_config(&RpcConfig {
        url: options.rpc_url(),
        auth: Auth::Cookie {
          file: options.cookie_file()?,
        },
        network: options.network,
        wallet_name: wallet_name_from_descriptor(
          Bip84((seed, None), KeychainKind::External),
          None,
          options.network,
          &Secp256k1::new(),
        )?,
        skip_blocks: None,
      })?,
      SyncOptions::default(),
    )?;

    eprintln!("Wallet initialized.");

    Ok(())
  }

  pub(crate) fn load(options: &Options) -> Result<Self> {
    let path = data_dir()
      .ok_or_else(|| anyhow!("Failed to retrieve data dir"))?
      .join("ord");

    if !path.exists() {
      return Err(anyhow!("Wallet doesn't exist."));
    }

    let key = (
      Mnemonic::from_entropy(&fs::read(path.join("entropy"))?)?,
      None,
    );

    let wallet = bdk::wallet::Wallet::new(
      Bip84(key.clone(), KeychainKind::External),
      None,
      options.network,
      SqliteDatabase::new(
        path
          .join("wallet.sqlite")
          .to_str()
          .ok_or_else(|| anyhow!("Failed to convert path to str"))?
          .to_string(),
      ),
    )?;

    let blockchain = RpcBlockchain::from_config(&RpcConfig {
      url: options.rpc_url(),
      auth: Auth::Cookie {
        file: options.cookie_file()?,
      },
      network: options.network,
      wallet_name: wallet_name_from_descriptor(
        Bip84(key, KeychainKind::External),
        None,
        options.network,
        &Secp256k1::new(),
      )?,
      skip_blocks: None,
    })?;

    wallet.sync(&blockchain, SyncOptions::default())?;

    Ok(Self { wallet, blockchain })
  }

  pub(crate) fn find(&self, options: &Options, ordinal: Ordinal) -> Result<LocalUtxo> {
    let index = Index::index(options)?;

    for utxo in self.wallet.list_unspent()? {
      if let Some(ranges) = index.list(utxo.outpoint)? {
        for (start, end) in ranges {
          if ordinal.0 >= start && ordinal.0 < end {
            return Ok(utxo);
          }
        }
      }
    }

    bail!("No utxo contains {}˚.", ordinal);
  }
}
