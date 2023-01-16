use super::*;

#[test]
fn restore_generates_same_descriptors() {
  let (mnemonic, descriptors) = {
    let rpc_server = test_bitcoincore_rpc::spawn();

    let Create { mnemonic } = CommandBuilder::new("wallet create")
      .rpc_server(&rpc_server)
      .output::<Create>();

    (mnemonic, rpc_server.descriptors())
  };

  let rpc_server = test_bitcoincore_rpc::spawn();

  CommandBuilder::new(["wallet", "restore", &mnemonic.to_string()])
    .rpc_server(&rpc_server)
    .run();

  assert_eq!(rpc_server.descriptors(), descriptors);
}
