use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::Secret;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::DatabaseClient;
use crate::internal::random::{generate_random_password, generate_username};
use crate::internal::vault::VaultClient;
use crate::internal::workflow::Workflow;
use crate::CLI_ARGS;

pub(crate) struct RotateWorkflow {
    argocd: ArgoCDClient,
    postgres: PostgresClient,
    vault: VaultClient,
}

impl Workflow for RotateWorkflow {
    fn new(argocd: ArgoCDClient, postgres: PostgresClient, vault: VaultClient) -> Self {
        return RotateWorkflow {
            argocd,
            postgres,
            vault,
        };
    }

    fn sanitize(&mut self, secrets: Vec<Secret>) -> Vec<Secret> {
        secrets
    }

    fn run(&mut self, secrets: Vec<Secret>) {
        let mut existing_users: Vec<String> = Vec::new();

        for secret in secrets {
            let prefix = secret.prefix;

            for existing_username in self.postgres.get_existing_users(&prefix) {
                existing_users.push(existing_username);
            }

            let username = generate_username(&prefix, secret.username_random_part_length);
            println!("Generated username for prefix '{}': {}", prefix, username);

            // TODO: Length from config
            let password = generate_random_password(12);
            println!("Generated password for prefix '{}': {}", prefix, password);

            self.postgres
                .create_user_and_assign_role(&username, &password, &secret.role);
            self.vault.update_username_and_password(
                username.as_str(),
                secret.username_key.as_str(),
                password.as_str(),
                secret.password_key.as_str(),
                secret.vault_path.as_str(),
            );
        }

        self.argocd.rollout_namespace();

        if !existing_users.is_empty() {
            self.postgres.drop_users(existing_users);
        } else {
            if CLI_ARGS.debug || CLI_ARGS.verbose {
                println!("🔎 No existing users present, will not cleanup");
            }
        }
    }
}
