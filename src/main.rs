extern crate config;
extern crate hashicorp_vault;
extern crate postgres;

use config::Config;
use config::File;
use hashicorp_vault::client::VaultClient;
use postgres::{Client, NoTls};
use rand::distributions::{Alphanumeric, DistString};
use serde_json::json;
use std::fs::File as FsFile;

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

fn create_user(username: &str, password: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let database_url = config.get_string("database_url")?;
    let mut client = Client::connect(&database_url, NoTls)?;
    client.execute(
        &format!("CREATE USER {} WITH PASSWORD '{}';", username, password),
        &[],
    )?;
    Ok(())
}

fn write_to_vault(username: &str, password: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let vault_url = config.get_string("vault_url")?;
    let vault_token = config.get_string("vault_token")?;
    let client = VaultClient::new(&vault_url, &vault_token)?;
    let secret_path = config.get_string("vault_secret_path")?;
    let secret_data = json!({
        "username": username,
        "password": password
    });
    client.set_secret(&secret_path, secret_data.to_string())?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if ".propellerrc" file exists
    let config_path = ".propellerrc";
    if !FsFile::open(&config_path).is_ok() {
        return Err(format!("Configuration file '{}' not found", &config_path).into());
    }

    // Load configuration from ".propellerrc" file
    let mut config = Config::default();
    config.merge(File::with_name(&config_path))?;

    let username = generate_username(&prefix, username_length);
    println!("Generated username: {}", username);

    let password = generate_random_password(12); // Generate a random password with 12 characters
    println!("Generated password: {}", password);

    create_user(&username, &password, &config)?;
    write_to_vault(&username, &password, &config)?;

    Ok(())
}
