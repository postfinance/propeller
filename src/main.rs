extern crate hashicorp_vault;
extern crate lazy_static;
extern crate serde_derive;

use clap::Parser;
use lazy_static::lazy_static;

use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::{Args, load_config};
use crate::internal::database::DatabaseClient;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::vault::VaultClient;
use crate::internal::workflow::rotate::RotateWorkflow;
use crate::internal::workflow::Workflow;

mod internal;

lazy_static! {
    pub(crate) static ref CLI_ARGS: Args = Args::parse();
}

fn main() {
    let config = load_config();

    let argocd = ArgoCDClient::new(&config.argocd);
    let postgres = PostgresClient::new(&config.database);
    let vault = VaultClient::new(&config.vault);

    if CLI_ARGS.workflow == "exchange" {
        let mut workflow = RotateWorkflow::new(argocd, postgres, vault);
        let secrets = workflow.sanitize(config.secrets);
        workflow.run(secrets);
    }
}
