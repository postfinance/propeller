extern crate postgres;
extern crate hashicorp_vault;

use postgres::{Client, NoTls};
use hashicorp_vault::client::VaultClient;
use serde_json::json;

fn create_user(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect("postgresql://your_username:your_password@your_host:your_port/your_database_name", NoTls)?;
    client.execute(
        &format!("CREATE USER {} WITH PASSWORD '{}';", username, password),
        &[],
    )?;
    Ok(())
}

fn write_to_vault(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = VaultClient::new("your_vault_url", "your_vault_token")?;
    let secret_path = "your_vault_secret_path";
    let secret_data = json!({
        "username": username,
        "password": password
    });
    client.set_secret(secret_path, secret_data.to_string())?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Enter username: ");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username)?;
    let username = username.trim();

    println!("Enter password: ");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    let password = password.trim();

    create_user(username, password)?;
    write_to_vault(username, password)?;

    Ok(())
}
