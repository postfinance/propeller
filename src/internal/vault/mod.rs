use std::collections::HashMap;
use std::process::exit;

use hashicorp_vault::client::{TokenData, VaultClient as HashiCorpVaultClient};
use serde_json::{json, to_string, Value};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct VaultConfig {
    url: String,
    token: String,
}

impl VaultConfig {
    pub fn new(url: &str, token: &str) -> Self {
        VaultConfig {
            url: url.to_string(),
            token: token.to_string(),
        }
    }
}

pub(crate) struct VaultClient {
    client: HashiCorpVaultClient<TokenData>,
}

impl VaultClient {
    pub(crate) fn new(argo_cd_config: &VaultConfig) -> Self {
        VaultClient {
            client: match HashiCorpVaultClient::new(&argo_cd_config.url, &argo_cd_config.token) {
                Ok(client) => client,
                Err(err) => {
                    eprintln!("🛑 Failed to initialize HashiCorp Vault client: {}", err);
                    exit(1)
                }
            },
        }
    }

    pub(crate) fn update_username_and_password(
        &mut self,
        username: &str,
        username_key: &str,
        password: &str,
        password_key: &str,
        secret_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut_client = &mut self.client;

        let existing_secret = read_existing_secret(secret_path, mut_client);
        let mut secret_data = json!(existing_secret);

        secret_data = modify_secret_data(secret_data, username_key, username);
        secret_data = modify_secret_data(secret_data, password_key, password);

        write_secret(secret_data, secret_path, mut_client);

        Ok(())
    }
}

fn read_existing_secret(secret_path: &str, client: &mut HashiCorpVaultClient<TokenData>) -> String {
    let secret = match client.get_secret(secret_path) {
        Ok(secret) => {
            println!(
                "✅ Existing secret successfully read from '{}'",
                secret_path
            );
            secret
        }
        Err(err) => {
            eprintln!("🛑 Failed to read secret from '{}': {}", secret_path, err);
            exit(1)
        }
    };
    secret
}

fn write_secret(
    secret_data: Value,
    secret_path: &str,
    client: &mut HashiCorpVaultClient<TokenData>,
) {
    match client.set_secret(secret_path, secret_data.to_string()) {
        Ok(_) => println!("✅ Secret successfully written to '{}'", secret_path),
        Err(err) => {
            eprintln!("🛑 Failed to write secret to '{}': {}", secret_path, err);
            exit(1)
        }
    };
}

fn modify_secret_data(mut data: Value, key: &str, value: &str) -> Value {
    if let Some(Value::String(old_value)) = data.get_mut(key) {
        *old_value = value.to_string();
    }
    data
}
