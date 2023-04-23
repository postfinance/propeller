extern crate hashicorp_vault;
extern crate postgres;
extern crate config;

use hashicorp_vault::client::VaultClient;
use postgres::{Client, NoTls};
use serde_json::json;
use config::Config;
use config::File;

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
    // Load configuration from rc file
    let mut config = Config::default();
    config.merge(File::with_name("config"))?; // Replace with your rc file name

    println!("Enter username: ");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username)?;
    let username = username.trim();

    println!("Enter password: ");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    let password = password.trim();

    create_user(username, password, &config)?;
    write_to_vault(username, password, &config)?;

    Ok(())
}
