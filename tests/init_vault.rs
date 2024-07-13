use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use predicates::str::contains;
use reqwest::{Client, Response};
use serde_json::Value;
use tokio::runtime::{Builder, Runtime};

mod common;

lazy_static! {
    static ref BIN_PATH: PathBuf = cargo_bin(env!("CARGO_PKG_NAME"));
}

#[test]
fn init_vault_new_path() {
    let vault_container = common::vault_container();

    let vault_host = vault_container.get_host().unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).unwrap();

    Command::new(&*BIN_PATH)
        .arg("init-vault")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
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
    let url = format!("http://{vault_host}:{vault_port}/v1/secret/data/init/vault/new/path");

    let rt: Runtime = create_tokio_runtime();
    let json = read_secret_as_json(client, url.as_str(), rt);

    assert_json_value_equals(&json, "postgresql_active_user", "TBD");
    assert_json_value_equals(&json, "postgresql_active_user_password", "TBD");
    assert_json_value_equals(&json, "postgresql_user_1", "TBD");
    assert_json_value_equals(&json, "postgresql_user_1_password", "TBD");
    assert_json_value_equals(&json, "postgresql_user_2", "TBD");
    assert_json_value_equals(&json, "postgresql_user_2_password", "TBD");
}

#[test]
fn init_vault_invalid_url() {
    common::vault_container();

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

fn create_tokio_runtime() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build Vault connection")
}

fn read_secret_as_json(client: Client, url: &str, rt: Runtime) -> Value {
    let response: Response = rt
        .block_on(client.get(url).header("X-Vault-Token", "root-token").send())
        .expect("Error receiving Vault data");

    response
        .error_for_status_ref()
        .expect("Expected to reach Vault");

    let json: Value = rt
        .block_on(response.json())
        .expect("Failed to convert Vault response to JSON");
    json
}

fn assert_json_value_equals(json: &Value, key: &str, value: &str) {
    assert_eq!(json["data"]["data"][key].as_str().unwrap(), value);
}
