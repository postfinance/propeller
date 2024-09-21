// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::process::{Command, Stdio};

use assert_cmd::prelude::*;
use ntest::timeout;
use predicates::str::contains;
use utilities::{
    create_vault_client, read_vault_secret, vault_container, write_string_to_tempfile,
};

#[tokio::test]
#[timeout(30_000)]
async fn init_vault_new_path() {
    let vault_container = vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    println!("Setup success; invoking propeller...");

    Command::cargo_bin("propeller")
        .unwrap()
        .arg("init-vault")
        .arg("-c")
        .arg(write_string_to_tempfile(
            format!(
                // language=yaml
                "
argo_cd:
  application: 'propeller'
  base_url: 'http://localhost:3100'
postgres:
  host: 'localhost'
  port: 5432
  database: 'demo'
vault:
  base_url: 'http://{vault_host}:{vault_port}'
  path: 'init/vault/new/path'
"
            )
            .as_str(),
        ))
        .env("VAULT_TOKEN", "root-token")
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .success()
        .stdout(contains(
            "Successfully initialized Vault path 'init/vault/new/path'",
        ));

    let vault_client = create_vault_client(vault_host.to_string().as_str(), vault_port);
    let vault_secret = read_vault_secret(&vault_client, "init/vault/new/path").await;

    assert_eq!(vault_secret.postgresql_active_user, "TBD");
    assert_eq!(vault_secret.postgresql_active_user_password, "TBD");
    assert_eq!(vault_secret.postgresql_user_1, "TBD");
    assert_eq!(vault_secret.postgresql_user_1_password, "TBD");
    assert_eq!(vault_secret.postgresql_user_2, "TBD");
    assert_eq!(vault_secret.postgresql_user_2_password, "TBD");
}

#[tokio::test]
#[timeout(30_000)]
async fn init_vault_invalid_url() {
    let _ = vault_container().await;

    println!("Setup success; invoking propeller...");

    Command::cargo_bin("propeller")
        .unwrap()
        .arg("init-vault")
        .arg("-c")
        .arg("tests/resources/init_vault/invalid_url.yml")
        .env("VAULT_TOKEN", "root-token")
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains("Failed to create initial Vault structure"))
        .stderr(contains("error sending request for url"));
}
