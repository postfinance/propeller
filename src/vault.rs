use log::{debug, info};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::runtime::{Builder, Runtime};
use vaultrs::api::kv2::responses::SecretVersionMetadata;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::error::ClientError;
use vaultrs::kv2;

use crate::config::{Config, VaultConfig};

const VAULT_TOKEN: &str = "VAULT_TOKEN";

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VaultStructure {
    pub(crate) postgresql_active_user: String,
    pub(crate) postgresql_active_user_password: String,
    pub(crate) postgresql_user_1: String,
    pub(crate) postgresql_user_1_password: String,
    pub(crate) postgresql_user_2: String,
    pub(crate) postgresql_user_2_password: String,
}

pub(crate) struct Vault {
    vault_client: VaultClient,
    vault_config: VaultConfig,
    rt: Runtime,
}

impl Vault {
    pub(crate) fn connect(config: &Config) -> Vault {
        debug!("Connecting to Vault at: {}", config.vault.base_url);

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

        info!("Initializing secret path '{}'", self.vault_config.path);

        let vault_structure = VaultStructure {
            postgresql_active_user: "TBD".to_string(),
            postgresql_active_user_password: "TBD".to_string(),
            postgresql_user_1: "TBD".to_string(),
            postgresql_user_1_password: "TBD".to_string(),
            postgresql_user_2: "TBD".to_string(),
            postgresql_user_2_password: "TBD".to_string(),
        };

        self.write_secret(&vault_structure)
            .expect("Failed to create initial Vault structure");

        println!(
            "Successfully initialized Vault path '{}'",
            self.vault_config.path
        )
    }

    pub(crate) fn read_secret<D: DeserializeOwned>(&mut self) -> Result<D, ClientError> {
        info!("Reading secret from path '{}'", self.vault_config.path);

        self.rt.block_on(kv2::read(
            &self.vault_client,
            "secret",
            &self.vault_config.path,
        ))
    }

    pub(crate) fn write_secret(
        &mut self,
        vault_structure: &VaultStructure,
    ) -> Result<SecretVersionMetadata, ClientError> {
        info!("Writing secret to path '{}'", self.vault_config.path);

        self.rt.block_on(kv2::set(
            &self.vault_client,
            "secret",
            &self.vault_config.path,
            &vault_structure,
        ))
    }
}

fn get_vault_client(config: &Config) -> VaultClient {
    let vault_token = env::var(VAULT_TOKEN).expect("Missing VAULT_TOKEN environment variable");

    let vault_client: VaultClient = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(config.vault.base_url.clone())
            .token(vault_token)
            .build()
            .unwrap(),
    )
    .unwrap();

    vault_client
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ArgoConfig, PostgresConfig};
    use vaultrs::client::Client;

    #[test]
    fn successful_vault_connect() {
        let config = Config {
            argo_cd: ArgoConfig {
                application: "sut".to_string(),
                base_url: "http://localhost:3100".to_string(),
                ..Default::default()
            },
            postgres: mock_postgres_config(),
            vault: VaultConfig {
                base_url: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::set_var(VAULT_TOKEN, "test_token"); // Mock environment variable

        let vault = Vault::connect(&config);

        assert_eq!(vault.vault_config.base_url, config.vault.base_url);
        assert_eq!(vault.vault_config.path, config.vault.path);
    }

    #[test]
    #[should_panic(expected = "Missing VAULT_TOKEN environment variable")]
    fn vault_connect_missing_token() {
        let config = Config {
            argo_cd: ArgoConfig {
                application: "sut".to_string(),
                base_url: "http://localhost:3100".to_string(),
                ..Default::default()
            },
            postgres: mock_postgres_config(),
            vault: VaultConfig {
                base_url: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::remove_var(VAULT_TOKEN); // Ensure VAULT_TOKEN is not present

        let _ = Vault::connect(&config); // This should panic
    }

    #[test]
    fn get_vault_client_returns_client() {
        let config = Config {
            argo_cd: ArgoConfig {
                application: "sut".to_string(),
                base_url: "http://localhost:3100".to_string(),
                ..Default::default()
            },
            postgres: mock_postgres_config(),
            vault: VaultConfig {
                base_url: "http://localhost:8200".to_string(),
                path: "path/to/my/secret".to_string(),
            },
        };
        env::set_var(VAULT_TOKEN, "test_token");

        let vault_client = get_vault_client(&config);

        assert_eq!(
            vault_client.settings().address.to_string(),
            config.vault.base_url + "/"
        );
    }

    fn mock_postgres_config() -> PostgresConfig {
        PostgresConfig {
            host: "".to_string(),
            port: 1234,
            database: "".to_string(),
        }
    }
}
