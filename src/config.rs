// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use log::debug;
use serde::Deserialize;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub(crate) argo_cd: ArgoConfig,
    pub(crate) postgres: PostgresConfig,
    pub(crate) vault: VaultConfig,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct ArgoConfig {
    pub(crate) application: String,
    pub(crate) base_url: String,
    pub(crate) danger_accept_insecure: Option<bool>,
    pub(crate) sync_timeout_seconds: Option<u16>,
}

impl Default for ArgoConfig {
    fn default() -> Self {
        ArgoConfig {
            application: String::from("propeller"),
            base_url: String::from("http://localhost:3100"),
            danger_accept_insecure: Option::from(false),
            sync_timeout_seconds: Option::from(60),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct PostgresConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) database: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        PostgresConfig {
            host: String::from("localhost"),
            port: 5432,
            database: String::from("propeller"),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct VaultConfig {
    pub(crate) base_url: String,
    pub(crate) path: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        VaultConfig {
            base_url: String::from("http://localhost:8200"),
            path: String::from("propeller"),
        }
    }
}

pub(crate) fn read_config(config_path: PathBuf) -> Config {
    let path_string = config_path.clone().into_os_string().into_string().unwrap();
    debug!("Reading config at: {path_string}");

    let mut config_data: String = String::new();
    let mut config_file: File = File::open(config_path)
        .unwrap_or_else(|e| panic!("Failed to read configuration file '{}': {}", path_string, e));
    config_file
        .read_to_string(&mut config_data)
        .expect("Failed to read configuration file");

    serde_yaml::from_str(&config_data).expect("Failed to parse configuration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(
        expected = "Failed to read configuration file 'tests/resources/config/non_existing.yml':" // OS agnostic error follows
    )]
    fn read_config_invalid_file() {
        read_config(PathBuf::from("tests/resources/config/non_existing.yml"));
    }

    #[test]
    #[should_panic(expected = "Failed to parse configuration: Error(\"missing field `argo_cd`")]
    fn read_config_missing_argo_cd() {
        read_config(PathBuf::from("tests/resources/config/missing_argo_cd.yml"));
    }

    #[test]
    #[should_panic(expected = "Failed to parse configuration: Error(\"missing field `postgres`")]
    fn read_config_missing_postgresql() {
        read_config(PathBuf::from(
            "tests/resources/config/missing_postgresql.yml",
        ));
    }

    #[test]
    #[should_panic(expected = "Failed to parse configuration: Error(\"missing field `vault`")]
    fn read_config_missing_vault() {
        read_config(PathBuf::from("tests/resources/config/missing_vault.yml"));
    }
}
