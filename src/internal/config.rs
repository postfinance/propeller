extern crate confy;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::internal::argocd::ArgoCDConfig;
use crate::internal::database::DatabaseConfig;
use crate::internal::vault::VaultConfig;

/// a secret rotation tool for applications running in Kubernetes, using HashiCorp Vault and ArgoCD.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Debug logging level
    #[arg(short, long, default_value_t = false)]
    pub(crate) debug: bool,

    /// Verbose logging level
    #[arg(short, long, default_value_t = false)]
    pub(crate) verbose: bool,

    /// Path to the configuration file
    #[arg(short, long)]
    pub(crate) config_path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Secret {
    pub(crate) password_key: String,
    pub(crate) prefix: String,
    pub(crate) role: String,
    pub(crate) username_key: String,
    pub(crate) username_random_part_length: usize,
    pub(crate) vault_path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    pub(crate) argocd: ArgoCDConfig,
    pub(crate) database: DatabaseConfig,
    pub(crate) vault: VaultConfig,
    pub(crate) secrets: Vec<Secret>,
}

// TODO: Shall respect given string path
// https://docs.rs/confy/latest/confy/fn.load_path.html
pub(crate) fn load_config(config_file_path: Option<&str>) -> Config {
    let config: Config;

    let error_message = "🛑 Failed to load propeller configuration!";

    if config_file_path.is_some() && !config_file_path.unwrap().is_empty() {
        config = confy::load_path(config_file_path.expect(error_message)).expect(error_message)
    } else {
        config = confy::load("propeller", None).expect(error_message);
    }

    config
}
