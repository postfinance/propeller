extern crate confy;

use serde::{Deserialize, Serialize};

use crate::internal::argocd::ArgoCDConfig;
use crate::internal::database::DatabaseConfig;
use crate::internal::vault::VaultConfig;

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
pub(crate) fn load_config() -> Config {
    let config: Config =
        confy::load("propeller", None).expect("🛑 Failed to load propeller configuration!");
    config
}
