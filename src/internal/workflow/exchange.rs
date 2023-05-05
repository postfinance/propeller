use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::Secret;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::DatabaseClient;
use crate::internal::random::{generate_random_password, generate_username};
use crate::internal::vault::VaultClient;
use crate::internal::workflow::Workflow;
use crate::CLI_ARGS;

pub(crate) struct ExchangeWorkflow {
    argocd: ArgoCDClient,
    postgres: PostgresClient,
    vault: VaultClient,
}

impl Workflow for ExchangeWorkflow {
    fn new(argocd: ArgoCDClient, postgres: PostgresClient, vault: VaultClient) -> Self {
        return ExchangeWorkflow {
            argocd,
            postgres,
            vault,
        };
    }

    fn sanitize(&mut self, secrets: Vec<Secret>) -> Vec<Secret> {
        todo!()
    }

    fn run(&mut self, secrets: Vec<Secret>) {
        todo!()
    }
}
