use std::env::temp_dir;
use std::fs::File;
use std::io::Write;

use testcontainers::{Container, ImageExt};
use testcontainers_modules::hashicorp_vault::HashicorpVault;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::SyncRunner;

pub(crate) fn postgres_container() -> Container<Postgres> {
    Postgres::default()
        .with_env_var("POSTGRES_DB", "demo")
        .with_env_var("POSTGRES_USER", "demo")
        .with_env_var("POSTGRES_PASSWORD", "demo_password")
        .start()
        .expect("PostgreSQL database started")
}

pub(crate) fn vault_container() -> Container<HashicorpVault> {
    HashicorpVault::default()
        .with_env_var("VAULT_DEV_ROOT_TOKEN_ID", "root-token")
        .start()
        .expect("Vault started")
}

pub(crate) fn write_string_to_tempfile(content: &str) -> String {
    let mut dir = temp_dir();
    let filename = format!("temp_file_{}", rand::random::<u64>());

    dir.push(filename);

    let mut file = File::create(dir.clone()).expect("Failed to create tmp file");

    file.write_all(content.as_bytes())
        .expect("Failed to write into tmp file");

    dir.to_string_lossy().to_string()
}
