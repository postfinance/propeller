use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::Secret;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::vault::VaultClient;

pub(crate) mod exchange;
pub(crate) mod rotate;

pub(crate) enum WorkflowKind {
    EXCHANGE,
    ROTATE,
}

impl std::str::FromStr for WorkflowKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "exchange" => Ok(WorkflowKind::EXCHANGE),
            "rotate" => Ok(WorkflowKind::ROTATE),
            _ => Err(format!("unknown color: {}", s)),
        }
    }
}

pub(crate) trait Workflow {
    fn new(argocd: ArgoCDClient, postgres: PostgresClient, vault: VaultClient) -> Self;
    fn sanitize(&mut self, secrets: Vec<Secret>) -> Vec<Secret>;
    fn run(&mut self, secrets: Vec<Secret>);
}
