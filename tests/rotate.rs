use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use postgres::NoTls;
use predicates::str::contains;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::runtime::{Builder, Runtime};

mod common;

lazy_static! {
    static ref BIN_PATH: PathBuf = cargo_bin(env!("CARGO_PKG_NAME"));
}

#[derive(Deserialize, Serialize)]
struct VaultSecret {
    postgresql_active_user: String,
    postgresql_active_user_password: String,
    postgresql_user_1: String,
    postgresql_user_1_password: String,
    postgresql_user_2: String,
    postgresql_user_2_password: String,
}

#[derive(Deserialize, Serialize)]
struct VaultSecretDTO {
    data: VaultSecret,
}

#[test]
fn rotate_secrets() {
    let vault_container = common::vault_container();

    let vault_host = vault_container.get_host().unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).unwrap();

    let postgres_container = common::postgres_container();

    let postgres_host = postgres_container.get_host().unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .unwrap()
        .to_string();

    let http_client = Client::new();
    let url = format!("http://{vault_host}:{vault_port}/v1/secret/data/rotate/secrets");

    let rt: Runtime = create_tokio_runtime();

    reset_vault_secret_path(&http_client, url.as_str(), &rt);

    let mut postgres_client = connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "demo",
        "demo_password",
    );
    reset_role_initial_password(&mut postgres_client, "user1");
    reset_role_initial_password(&mut postgres_client, "user2");

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
postgres:
  host: '{postgres_host}'
  port: {postgres_port}
  database: 'demo'
vault:
  address: 'http://{vault_host}:{vault_port}'
  path: 'rotate/secrets'
                "
            )
            .as_str(),
        ))
        .env("VAULT_TOKEN", "root-token")
        .assert()
        .success()
        .stdout(contains("Successfully rotated all secrets"));

    let json = read_secret_as_json(http_client, url.as_str(), rt);

    assert_eq!(
        json["data"]["data"]["postgresql_active_user"]
            .as_str()
            .unwrap(),
        "user2"
    );
    assert_ne!(
        json["data"]["data"]["postgresql_active_user_password"]
            .as_str()
            .unwrap(),
        "initialpw"
    );
    assert_eq!(
        json["data"]["data"]["postgresql_user_1"].as_str().unwrap(),
        "user1"
    );
    assert_ne!(
        json["data"]["data"]["postgresql_user_1_password"]
            .as_str()
            .unwrap(),
        "initialpw"
    );
    assert_eq!(
        json["data"]["data"]["postgresql_user_2"].as_str().unwrap(),
        "user2"
    );
    assert_ne!(
        json["data"]["data"]["postgresql_user_2_password"]
            .as_str()
            .unwrap(),
        "initialpw"
    );

    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user1",
        json["data"]["data"]["postgresql_user_1_password"]
            .as_str()
            .unwrap(),
    );
    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user2",
        json["data"]["data"]["postgresql_user_2_password"]
            .as_str()
            .unwrap(),
    );
}

#[test]
fn rotate_invalid_initialized_secret() {
    let vault_container = common::vault_container();

    let vault_host = vault_container.get_host().unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).unwrap();

    let postgres_container = common::postgres_container();

    let postgres_host = postgres_container.get_host().unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .unwrap()
        .to_string();

    let http_client = Client::new();
    let url = format!(
        "http://{vault_host}:{vault_port}/v1/secret/data/rotate/invalid/initialized/secret"
    );

    let rt: Runtime = create_tokio_runtime();
    create_invalid_vault_secret_path(&http_client, url.as_str(), &rt);

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
postgres:
  host: '{postgres_host}'
  port: {postgres_port}
  database: 'demo'
vault:
  address: 'http://{vault_host}:{vault_port}'
  path: 'rotate/invalid/initialized/secret'
                "
            )
            .as_str(),
        ))
        .env("VAULT_TOKEN", "root-token")
        .assert()
        .failure()
        .stderr(contains(
            "Failed to detect active user - did neither match user 1 nor 2",
        ));
}

#[test]
fn rotate_non_existing_secret() {
    let vault_container = common::vault_container();

    let vault_host = vault_container.get_host().unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).unwrap();

    let postgres_container = common::postgres_container();

    let postgres_host = postgres_container.get_host().unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .unwrap()
        .to_string();

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
postgres:
  host: '{postgres_host}'
  port: {postgres_port}
  database: 'demo'
vault:
  address: 'http://{vault_host}:{vault_port}'
  path: 'rotate/non/existing/path'
                "
            )
            .as_str(),
        ))
        .env("VAULT_TOKEN", "root-token")
        .assert()
        .failure()
        .stderr(contains(
            "Failed to read path 'rotate/non/existing/path' - did you init Vault?",
        ));
}

fn create_tokio_runtime() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build Vault connection")
}

fn reset_vault_secret_path(client: &Client, url: &str, rt: &Runtime) {
    let initial_secret = VaultSecretDTO {
        data: VaultSecret {
            postgresql_active_user: "user1".to_string(),
            postgresql_active_user_password: "initialpw".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "initialpw".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "initialpw".to_string(),
        },
    };

    write_vault_secret(client, url, rt, &initial_secret);
}

fn create_invalid_vault_secret_path(client: &Client, url: &str, rt: &Runtime) {
    let initial_secret = VaultSecretDTO {
        data: VaultSecret {
            postgresql_active_user: "userX".to_string(), // Note that 'userX' does neither match 'user1' nor 'user2'
            postgresql_active_user_password: "initialpw".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "initialpw".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "initialpw".to_string(),
        },
    };

    write_vault_secret(client, url, rt, &initial_secret);
}

fn write_vault_secret(client: &Client, url: &str, rt: &Runtime, initial_secret: &VaultSecretDTO) {
    rt.block_on(
        client
            .post(url)
            .header("X-Vault-Token", "root-token")
            .json(&initial_secret)
            .send(),
    )
    .expect("Error initializing Vault for 'rotate_secrets'");
}

fn connect_postgres_client(host: &str, port: &str, user: &str, password: &str) -> postgres::Client {
    let postgres_client = postgres::Client::connect(
        format!("host={host} port={port} dbname=demo user={user} password={password}").as_str(),
        NoTls,
    )
    .expect("Failed to build PostgreSQL connection");
    postgres_client
}

fn reset_role_initial_password(postgres_client: &mut postgres::Client, role: &str) {
    match postgres_client.execute(
        format!("CREATE USER {role} WITH PASSWORD 'initialpw'").as_str(),
        &[],
    ) {
        Ok(_) => {}
        Err(_) => {
            postgres_client
                .execute(
                    format!("ALTER ROLE {role} WITH PASSWORD 'initialpw'").as_str(),
                    &[],
                )
                .expect(format!("Failed to reset '{role}'").as_str());
        }
    }
}

fn read_secret_as_json(http_client: Client, url: &str, rt: Runtime) -> Value {
    let response: Response = rt
        .block_on(
            http_client
                .get(url)
                .header("X-Vault-Token", "root-token")
                .send(),
        )
        .expect("Error receiving Vault data");

    response
        .error_for_status_ref()
        .expect("Expected to reach Vault");

    let json: Value = rt
        .block_on(response.json())
        .expect("Failed to convert Vault response to JSON");
    json
}
