extern crate config;
extern crate hashicorp_vault;

use std::fs::File as FsFile;
use std::process::exit;

use config::Config;
use config::File;
use rand::distributions::{Alphanumeric, DistString};

use crate::internal::argocd::{ArgoCDClient, ArgoCDConfig};
use crate::internal::database::postgres::PostgresClient;
use crate::internal::database::{DatabaseClient, DatabaseConfig};
use crate::internal::vault::{VaultClient, VaultConfig};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if ".propellerrc" file exists
    let config_path = ".propellerrc";
    if !FsFile::open(&config_path).is_ok() {
        return Err(format!("Configuration file '{}' not found", &config_path).into());
    }

    // Load configuration from ".propellerrc" file
    let mut config = Config::default();
    // TODO: use `ConfigBuilder` instead
    config.merge(File::with_name(&config_path))?;

    // TODO: Could be [postgres] block
    let mut postgres = PostgresClient::new(&DatabaseConfig::new(
        config.get_string("database_url")?.as_str(),
    ));

    // TODO: Could be [argocd] block
    let mut argocd = ArgoCDClient::new(&ArgoCDConfig::new(
        config.get_string("argocd_url")?.as_str(),
        config.get_string("argocd_namespace")?.as_str(),
        config.get_string("argocd_token")?.as_str(),
    ));

    // TODO: Could be [vault] block
    let mut vault = VaultClient::new(&VaultConfig::new(
        config.get_string("vault_url")?.as_str(),
        config.get_string("vault_token")?.as_str(),
    ));

    let mut existing_users: Vec<String> = Vec::new();

    let username_map = config.get_array("secrets")?; // Read username map from configuration
    for secret in username_map {
        let secret_config = match secret.into_table() {
            Ok(cfg) => cfg,
            Err(err) => {
                println!("Failed to load configuration: {}", err);
                exit(1)
            }
        };

        // TODO: `unwrap` is unsafe!
        let prefix = secret_config.get("prefix").unwrap().to_string();
        let role = secret_config.get("role").unwrap().to_string();
        let secret_path = secret_config.get("vault_path").unwrap().to_string();

        for existing_username in postgres.get_existing_users(&prefix)? {
            existing_users.push(existing_username);
        }

        let username_length = config.get_int("username_length")? as usize;
        let username = generate_username(&prefix, username_length);
        println!("Generated username for prefix '{}': {}", prefix, username);

        let password = generate_random_password(12); // Generate a random password with 12 characters
        println!("Generated password for prefix '{}': {}", prefix, password);

        let username_key = secret_config.get("username_key").unwrap().to_string();
        let password_key = secret_config.get("password_key").unwrap().to_string();

        postgres.create_user_and_assign_role(&username, &password, &role)?;
        vault.update_username_and_password(
            username.as_str(),
            username_key.as_str(),
            password.as_str(),
            password_key.as_str(),
            secret_path.as_str(),
        )?;
    }

    argocd.sync_namespace()?;

    // Delete users from PostgreSQL database if any existing users were found
    if !existing_users.is_empty() {
        postgres.drop_users(existing_users)?;
    }

    Ok(())
}
