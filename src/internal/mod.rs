use serde::{Deserialize, Serialize};

use crate::internal::argocd::ArgoCDConfig;
use crate::internal::database::DatabaseConfig;
use crate::internal::vault::VaultConfig;

pub(crate) mod argocd;
pub(crate) mod database;
pub(crate) mod vault;

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
