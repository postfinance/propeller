use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use ntest::timeout;
use predicates::str::contains;
use reqwest::{Client, Response};
use serde_json::Value;

mod common;

lazy_static! {
    static ref BIN_PATH: PathBuf = cargo_bin(env!("CARGO_PKG_NAME"));
}

#[tokio::test]
#[timeout(30_000)]
async fn init_vault_new_path() {
    let vault_container = common::vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    Command::new(&*BIN_PATH)
        .arg("init-vault")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
argo_cd:
  application: 'sut'
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
        .assert()
        .success()
        .stdout(contains(
            "Successfully initialized Vault path 'init/vault/new/path'",
        ));

    let client = Client::new();

    let vault_url = format!("http://{vault_host}:{vault_port}/v1/secret/data/init/vault/new/path");
    let json_secret = read_secret_as_json(client, vault_url.as_str()).await;

    assert_json_value_equals(&json_secret, "postgresql_active_user", "TBD");
    assert_json_value_equals(&json_secret, "postgresql_active_user_password", "TBD");
    assert_json_value_equals(&json_secret, "postgresql_user_1", "TBD");
    assert_json_value_equals(&json_secret, "postgresql_user_1_password", "TBD");
    assert_json_value_equals(&json_secret, "postgresql_user_2", "TBD");
    assert_json_value_equals(&json_secret, "postgresql_user_2_password", "TBD");
}

#[tokio::test]
#[timeout(30_000)]
async fn init_vault_invalid_url() {
    common::vault_container().await;

    Command::new(&*BIN_PATH)
        .arg("init-vault")
        .arg("-c")
        .arg("tests/resources/init_vault/invalid_url.yml")
        .env("VAULT_TOKEN", "root-token")
        .assert()
        .failure()
        .stderr(contains("Failed to create initial Vault structure"))
        .stderr(contains("error sending request for url"));
}

async fn read_secret_as_json(client: Client, url: &str) -> Value {
    let response: Response = client
        .get(url)
        .header("X-Vault-Token", "root-token")
        .send()
        .await
        .expect("Error receiving Vault data");

    response
        .error_for_status_ref()
        .expect("Expected to reach Vault");

    let json: Value = response
        .json()
        .await
        .expect("Failed to convert Vault response to JSON");

    json
}

fn assert_json_value_equals(json: &Value, key: &str, value: &str) {
    assert_eq!(json["data"]["data"][key].as_str().unwrap(), value);
}
