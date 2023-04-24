extern crate confy;
extern crate hashicorp_vault;
extern crate serde_derive;

use rand::distributions::{Alphanumeric, DistString};

use crate::internal::argocd::ArgoCDClient;
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::DatabaseClient;
use crate::internal::vault::VaultClient;
use crate::internal::Config;

mod internal;

fn generate_username(prefix: &str, length: usize) -> String {
    let random_part = Alphanumeric.sample_string(&mut rand::thread_rng(), length);
    format!("{}{}", prefix, random_part)
}

/**
 * **Note:** In principle, all RNGs in Rand implementing CryptoRng are suitable as a source of
 * randomness for generating passwords (if they are properly seeded), but it is more conservative to
 * only use randomness directly from the operating system via the getrandom crate, or the
 * corresponding bindings of a crypto library.
 *
 * Source: https://rust-random.github.io/rand/rand/distributions/struct.Alphanumeric.html#passwords.
 */
fn generate_random_password(length: usize) -> String {
    let password = Alphanumeric.sample_string(&mut rand::thread_rng(), length);
    password
}

fn main() {
    let config: Config =
        confy::load("propeller", None).expect("🛑 Failed to load propeller configuration!");

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
    }
}
