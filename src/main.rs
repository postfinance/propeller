extern crate hashicorp_vault;
extern crate lazy_static;
extern crate serde_derive;

use clap::Parser;
use lazy_static::lazy_static;

use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::{load_config, Args};
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::DatabaseClient;
use crate::internal::vault::VaultClient;
use crate::internal::workflow::rotate::RotateWorkflow;
use crate::internal::workflow::{Workflow, WorkflowKind};

mod internal;

lazy_static! {
    pub(crate) static ref CLI_ARGS: Args = Args::parse();
}

fn main() {
    let config = load_config();

    let argocd = ArgoCDClient::new(&config.argocd);
    let postgres = PostgresClient::new(&config.database);
    let vault = VaultClient::new(&config.vault);

    let mut workflow;

    match CLI_ARGS
        .workflow
        .to_string()
        .parse::<WorkflowKind>()
        .expect(
            format!(
                "🛑 Invalid workflow '{}', allowed values are: exchange, rotate!",
                CLI_ARGS                    .workflow
            )
            .as_str(),
        ) {
        WorkflowKind::EXCHANGE => workflow = RotateWorkflow::new(argocd, postgres, vault),
        WorkflowKind::ROTATE => workflow = RotateWorkflow::new(argocd, postgres, vault),
    }

    let secrets = workflow.sanitize(config.secrets);
    workflow.run(secrets);
}
