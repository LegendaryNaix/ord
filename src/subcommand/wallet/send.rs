use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Send {
  satpoint: SatPoint,
  address: Address,
}

impl Send {
  pub(crate) fn run(self, options: Options) -> Result {
    let client = options.bitcoin_rpc_client_for_wallet_command("ord wallet send")?;

    if !self.address.is_valid_for_network(options.chain.network()) {
      bail!(
        "Address `{}` is not valid for {}",
        self.address,
        options.chain
      );
    }

    let index = Index::open(&options)?;
    index.update()?;

    let utxos = list_utxos(&options)?;

    let inscription_satpoints = index.get_inscription_satpoints()?;

    let change = get_change_addresses(&options, 2)?;

    let unsigned_transaction = TransactionBuilder::build_transaction(
      self.satpoint,
      inscription_satpoints,
      utxos,
      self.address,
      change,
    )?;

    let signed_tx = client
      .sign_raw_transaction_with_wallet(&unsigned_transaction, None, None)?
      .hex;

    let txid = client.send_raw_transaction(&signed_tx)?;

    println!("{txid}");
    Ok(())
  }
}
