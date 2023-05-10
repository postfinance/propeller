extern crate confy;

use crate::CLI_ARGS;
use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::internal::argocd::ArgoCDConfig;
use crate::internal::database::DatabaseConfig;
use crate::internal::vault::VaultConfig;

/// A secret rotation tool for applications running in Kubernetes, using HashiCorp Vault and ArgoCD.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Prevents all side effects from being executed
    #[arg(short, long, default_value_t = false)]
    pub(crate) dry_run: bool,

    /// Print debugging information
    #[arg(short, long, default_value_t = false)]
    pub(crate) debug: bool,

    /// Print everything available, **including sensitive information!**
    #[arg(short, long, default_value_t = false)]
    pub(crate) verbose: bool,

    /// Path to the configuration file
    #[arg(short, long)]
    pub(crate) config_path: String,

    /// The workflow to use; either exchange or rotate
    #[arg(short, long, default_value = "exchange")]
    pub(crate) workflow: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Secret {
    pub(crate) password_key: String,
    pub(crate) password_2_key: String,
    pub(crate) prefix: String,
    pub(crate) role: String,
    pub(crate) username_key: String,
    pub(crate) username_2_key: String,
    // TODO: This should be a global property
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

pub(crate) fn load_config() -> Config {
    let config: Config;

    let error_message = "🛑 Failed to load propeller configuration!";

    let config_path = CLI_ARGS.config_path.to_string();

    if !config_path.is_empty() {
        config = confy::load_path(config_path).expect(error_message)
    } else {
        config = confy::load("propeller", None).expect(error_message);
    }

    config
}
