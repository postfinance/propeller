use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::Secret;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::vault::VaultClient;

pub(crate) mod exchange;
pub(crate) mod rotate;

pub(crate) trait Workflow {
    fn new(argocd: ArgoCDClient, postgres: PostgresClient, vault: VaultClient) -> Self;
    fn sanitize(&mut self, secrets: Vec<Secret>) -> Vec<Secret>;
    fn run(&mut self, secrets: Vec<Secret>);
}
