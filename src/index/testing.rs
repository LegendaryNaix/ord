use super::*;

pub(crate) struct ContextBuilder {
  args: Vec<OsString>,
  chain: Chain,
  tempdir: Option<TempDir>,
}

impl ContextBuilder {
  pub(crate) fn build(self) -> Context {
    self.try_build().unwrap()
  }

  pub(crate) fn try_build(self) -> Result<Context> {
    let rpc_server = test_bitcoincore_rpc::builder()
      .network(self.chain.network())
      .build();

    let tempdir = self.tempdir.unwrap_or_else(|| TempDir::new().unwrap());
    let cookie_file = tempdir.path().join("cookie");
    fs::write(&cookie_file, "username:password").unwrap();

    let command: Vec<OsString> = vec![
      "ord".into(),
      "--rpc-url".into(),
      rpc_server.url().into(),
      "--data-dir".into(),
      tempdir.path().into(),
      "--cookie-file".into(),
      cookie_file.into(),
      format!("--chain={}", self.chain).into(),
    ];

    let options = Options::try_parse_from(command.into_iter().chain(self.args)).unwrap();
    let index = Index::open(&options)?;
    index.update().unwrap();

    Ok(Context {
      options,
      rpc_server,
      tempdir,
      index,
    })
  }

  pub(crate) fn arg(mut self, arg: impl Into<OsString>) -> Self {
    self.args.push(arg.into());
    self
  }

  pub(crate) fn args<T: Into<OsString>, I: IntoIterator<Item = T>>(mut self, args: I) -> Self {
    self.args.extend(args.into_iter().map(|arg| arg.into()));
    self
  }

  pub(crate) fn tempdir(mut self, tempdir: TempDir) -> Self {
    self.tempdir = Some(tempdir);
    self
  }
}

pub(crate) struct Context {
  pub(crate) options: Options,
  pub(crate) rpc_server: test_bitcoincore_rpc::Handle,
  #[allow(unused)]
  pub(crate) tempdir: TempDir,
  pub(crate) index: Index,
}

impl Context {
  pub(crate) fn builder() -> ContextBuilder {
    ContextBuilder {
      args: Vec::new(),
      tempdir: None,
      chain: Chain::Regtest,
    }
  }

  pub(crate) fn mine_blocks(&self, n: u64) -> Vec<Block> {
    let blocks = self.rpc_server.mine_blocks(n);
    self.index.update().unwrap();
    blocks
  }

  pub(crate) fn mine_blocks_with_subsidy(&self, n: u64, subsidy: u64) -> Vec<Block> {
    let blocks = self.rpc_server.mine_blocks_with_subsidy(n, subsidy);
    self.index.update().unwrap();
    blocks
  }

  pub(crate) fn configurations() -> Vec<Context> {
    vec![
      Context::builder().build(),
      Context::builder().arg("--index-sats").build(),
    ]
  }
}
