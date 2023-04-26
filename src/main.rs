extern crate hashicorp_vault;
extern crate lazy_static;
extern crate serde_derive;

use clap::Parser;
use lazy_static::lazy_static;

use crate::internal::argocd::ArgoCDClient;
use crate::internal::config::{load_config, Args};
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::DatabaseClient;
use crate::internal::random::{generate_random_password, generate_username};
use crate::internal::vault::VaultClient;

mod internal;

lazy_static! {
    pub(crate) static ref CLI_ARGS: Args = Args::parse();
}

fn main() {
    let config = load_config();

    let mut argocd = ArgoCDClient::new(&config.argocd);
    let mut postgres = PostgresClient::new(&config.database);
    let mut vault = VaultClient::new(&config.vault);

    let mut existing_users: Vec<String> = Vec::new();

    for secret in config.secrets {
        let prefix = secret.prefix;

        for existing_username in postgres.get_existing_users(&prefix) {
            existing_users.push(existing_username);
        }

        let username = generate_username(&prefix, secret.username_random_part_length);
        println!("Generated username for prefix '{}': {}", prefix, username);

        let password = generate_random_password(12);
        println!("Generated password for prefix '{}': {}", prefix, password);

        postgres.create_user_and_assign_role(&username, &password, &secret.role);
        vault.update_username_and_password(
            username.as_str(),
            secret.username_key.as_str(),
            password.as_str(),
            secret.password_key.as_str(),
            secret.vault_path.as_str(),
        );
    }

    argocd.rollout_namespace();

    if !existing_users.is_empty() {
        postgres.drop_users(existing_users);
    } else {
        if CLI_ARGS.debug || CLI_ARGS.verbose {
            println!("🔎 No existing users present, will not cleanup");
        }
    }
}
