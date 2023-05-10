use hashicorp_vault::client::{TokenData, VaultClient as HashiCorpVaultClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::exit;

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

    pub(crate) fn read_secret(&mut self, secret_path: &str) -> Value {
        let client = &mut self.client;
        let secret = json!(read_existing_secret(secret_path, client));
        if !secret.is_object() {
            eprintln!("🛑 Secret value is not a valid JSON object!");
            exit(1)
        }
        secret
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

        let existing_secret = self.read_secret(secret_path);
        let mut secret_data = json!(existing_secret);

        modify_secret_data(&mut secret_data, username_key, username);
        modify_secret_data(&mut secret_data, password_key, password);

        write_secret(secret_data, secret_path, client);
    }

    pub(crate) fn exchange_active_username_and_password(
        &mut self,
        active_username_key: &str,
        active_password_key: &str,
        passive_username_key: &str,
        passive_password_key: &str,
        new_active_user_password: &str,
        secret_path: &str,
    ) {
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

fn modify_secret_data(json_object: &mut Value, key: &str, value: &str) {
    let mut_json = *json_object;
    if let Value::Object(mut object) = mut_json {
        object.insert(key.to_string(), json!(value));
    }

    if CLI_ARGS.verbose {
        println!("👀 Modified key '{}' successfully: '{}", key, value);
    }
}
