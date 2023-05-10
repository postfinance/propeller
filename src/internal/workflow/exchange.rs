use serde_json::{json, Value};
use std::fs::read;
use std::ops::Deref;
use std::process::exit;

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
        secrets
    }

    fn run(&mut self, secrets: Vec<Secret>) {
        for secret in secrets {
            let mut vault_secret = self.vault.read_secret(secret.vault_path.as_str());

            // TODO: Length from config
            let new_active_user_password = generate_random_password(12).as_str();

            if let Some(soon_to_be_active_username) =
                read_json_key(&mut vault_secret, secret.username_2_key.as_str())
                    .as_ref()
                    .and_then(|v| v.as_str())
            {
                self.postgres.update_user_password(
                    soon_to_be_active_username.as_str(),
                    new_active_user_password,
                );
            } else {
                eprintln!(
                    "🛑 Failed to update password of passive user from key '{}' in secret '{}'",
                    secret.username_2_key.as_str(),
                    secret.vault_path.as_str()
                );
                exit(1);
            }

            self.vault.exchange_active_username_and_password(
                secret.username_key.as_str(),
                secret.password_key.as_str(),
                secret.username_2_key.as_str(),
                secret.password_2_key.as_str(),
                new_active_user_password,
                secret.vault_path.as_str(),
            );

            self.argocd.rollout_namespace();

            if let Some(now_passive_username) =
                read_json_key(&mut vault_secret, secret.username_2_key.as_str())
                    .as_ref()
                    .and_then(|v| v.as_str())
            {
                self.vault.update_username_and_password(
                    now_passive_username,
                    secret.username_2_key.as_str(),
                    // TODO: Length from config
                    generate_random_password(12).as_str(),
                    secret.password_2_key.as_str(),
                    secret.vault_path.as_str(),
                );
            } else {
                eprintln!(
                    "🛑 Failed to read passive username key '{}' in secret '{}'",
                    secret.username_2_key.as_str(),
                    secret.vault_path.as_str()
                );
                exit(1);
            }
        }
    }
}

fn read_json_key<'a>(json: &'a mut Value, key: &str) -> &'a mut Value {
    json.get_mut(key)
        .expect(format!("🛑 Failed to read secret key '{}'!", key).as_str())
}
