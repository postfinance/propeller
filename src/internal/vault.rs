use hashicorp_vault::client::{TokenData, VaultClient as HashiCorpVaultClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::CLI_ARGS;

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
            client: HashiCorpVaultClient::new(&argo_cd_config.url, &argo_cd_config.token)
                .expect("🛑 Failed to initialize HashiCorp Vault client!"),
        }
    }

    pub(crate) fn update_username_and_password(
        &mut self,
        username: &str,
        username_key: &str,
        password: &str,
        password_key: &str,
        secret_path: &str,
    ) {
        if CLI_ARGS.dry_run {
            println!("🧪 Would now update secret in path '{}'", secret_path);
            println!("🧪 Username key '{}' -> '{}'", username_key, username);
            println!("🧪 Password key '{}' -> '{}'", password_key, password);
            return;
        }

        let client = &mut self.client;

        let existing_secret = read_existing_secret(secret_path, client);
        let mut secret_data = json!(existing_secret);

        secret_data = modify_secret_data(secret_data, username_key, username);
        secret_data = modify_secret_data(secret_data, password_key, password);

        write_secret(secret_data, secret_path, client);
    }
}

fn read_existing_secret(secret_path: &str, client: &mut HashiCorpVaultClient<TokenData>) -> String {
    if CLI_ARGS.verbose {
        println!("👀 Gonna read secret in path '{}'", secret_path);
    }

    let secret = client
        .get_secret(secret_path)
        .expect(format!("🛑 Failed to read secret from '{}'!", secret_path).as_str());

    if CLI_ARGS.debug {
        println!(
            "🔎 Existing secret successfully read from '{}'",
            secret_path
        );
    } else if CLI_ARGS.verbose {
        println!(
            "👀 Existing secret successfully read from '{}': {}",
            secret_path, secret
        );
    }

    secret
}

fn write_secret(
    secret_data: Value,
    secret_path: &str,
    client: &mut HashiCorpVaultClient<TokenData>,
) {
    if CLI_ARGS.debug {
        println!("🔎 Updating secret in path '{}'", secret_path);
    } else if CLI_ARGS.verbose {
        println!(
            "👀 Updating secret in path '{}': {}",
            secret_path,
            secret_data.to_string()
        );
    }

    client
        .set_secret(secret_path, secret_data.to_string())
        .expect(format!("🛑 Failed to read secret from '{}'!", secret_path).as_str());

    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!(
            "🔎 Updated secret successfully written to '{}'",
            secret_path
        );
    }
}

fn modify_secret_data(mut data: Value, key: &str, value: &str) -> Value {
    if let Some(Value::String(old_value)) = data.get_mut(key) {
        *old_value = value.to_string();
    }

    if CLI_ARGS.verbose {
        println!("👀 Modified key '{}' successfully: '{}", key, value);
    }

    data
}
