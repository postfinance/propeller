use log::info;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::runtime::{Builder, Runtime};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

use crate::config::{Config, VaultConfig};

const VAULT_TOKEN: &'static str = "VAULT_TOKEN";

#[derive(Debug, Deserialize, Serialize)]
struct VaultStructure {
    postgresql_active_user: String,
    postgresql_active_user_password: String,
    postgresql_user_1: String,
    postgresql_user_1_password: String,
    postgresql_user_2: String,
    postgresql_user_2_password: String,
}

pub(crate) struct Vault {
    vault_client: VaultClient,
    vault_config: VaultConfig,
    rt: Runtime,
}

impl Vault {
    pub(crate) fn connect(config: &Config) -> Vault {
        Vault {
            vault_client: get_vault_client(config),
            vault_config: config.vault.clone(),
            rt: Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Vault connection"),
        }
    }

    pub(crate) fn init_secret_path(&mut self) {
        // TODO: Theoretically it would be possible to check if anything exists in this path already - exit if so.

        info!("Initializing secret path");

        let vault_structure = VaultStructure {
            postgresql_active_user: "TBD".to_string(),
            postgresql_active_user_password: "TBD".to_string(),
            postgresql_user_1: "TBD".to_string(),
            postgresql_user_1_password: "TBD".to_string(),
            postgresql_user_2: "TBD".to_string(),
            postgresql_user_2_password: "TBD".to_string(),
        };

        self.rt
            .block_on(kv2::set(
                &self.vault_client,
                "secret",
                &*self.vault_config.path,
                &vault_structure,
            ))
            .expect("Failed to create initial Vault structure");

        println!(
            "Successfully initialized Vault path '{}'",
            self.vault_config.path
        )
    }
}

fn get_vault_client(config: &Config) -> VaultClient {
    let vault_token = env::var(VAULT_TOKEN).expect("Missing VAULT_TOKEN environment variable");

    let vault_client: VaultClient = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(config.vault.address.clone())
            .token(vault_token)
            .build()
            .unwrap(),
    )
    .unwrap();

    vault_client
}

mod test {
    use super::*;
    use crate::config::PostgresConfig;
    use vaultrs::client::Client;

    #[test]
    fn test_vault_connect() {
        let config = Config {
            postgres: PostgresConfig {
                jdbc_url: "".to_string(),
            },
            vault: VaultConfig {
                address: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::set_var(VAULT_TOKEN, "test_token"); // Mock environment variable

        let vault = Vault::connect(&config);

        assert_eq!(vault.vault_config.address, config.vault.address);
        assert_eq!(vault.vault_config.path, config.vault.path);
    }

    #[test]
    #[should_panic(expected = "Missing VAULT_TOKEN environment variable")]
    fn test_vault_connect_missing_token() {
        let config = Config {
            postgres: PostgresConfig {
                jdbc_url: "".to_string(),
            },
            vault: VaultConfig {
                address: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::remove_var(VAULT_TOKEN); // Ensure VAULT_TOKEN is not present

        let _ = Vault::connect(&config); // This should panic
    }

    #[test]
    fn test_get_vault_client() {
        let config = Config {
            postgres: PostgresConfig {
                jdbc_url: "".to_string(),
            },
            vault: VaultConfig {
                address: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::set_var(VAULT_TOKEN, "test_token");

        let vault_client = get_vault_client(&config);

        assert_eq!(
            vault_client.settings().address.to_string(),
            config.vault.address + "/"
        );
    }
}
