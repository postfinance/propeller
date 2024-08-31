use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use base64::engine::general_purpose;
use base64::Engine;
use lazy_static::lazy_static;
use ntest::timeout;
use postgres::NoTls;
use predicates::str::contains;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::str::FromStr;

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

#[tokio::test]
#[timeout(120_000)]
async fn rotate_secrets() {
    deploy_argocd();

    let postgres_container = common::postgres_container().await;

    let postgres_host = postgres_container.get_host().await.unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .await
        .unwrap()
        .to_string();

    let vault_container = common::vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    let http_client = Client::new();

    let vault_url = format!("http://{vault_host}:{vault_port}/v1/secret/data/rotate/secrets");
    reset_vault_secret_path(&http_client, vault_url.as_str()).await;

    let mut postgres_client = connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "demo",
        "demo_password",
    )
    .await;

    reset_role_initial_password(&mut postgres_client, "user1").await;
    reset_role_initial_password(&mut postgres_client, "user2").await;

    let (argocd_port, mut port_forward) = open_argocd_server_port_forward();
    let argocd_url = format!("http://localhost:{}", argocd_port);

    let argocd_token = get_argocd_access_token(argocd_url.as_str()).await;
    create_argocd_application(argocd_url.as_str(), argocd_token.as_str()).await;

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
    argo_cd:
      application: 'propeller'
      danger_accept_insecure: true
      base_url: 'http://127.0.0.1:{argocd_port}'
    postgres:
      host: '{postgres_host}'
      port: {postgres_port}
      database: 'demo'
    vault:
      base_url: 'http://{vault_host}:{vault_port}'
      path: 'rotate/secrets'
"
            )
            .as_str(),
        ))
        .env("ARGO_CD_TOKEN", argocd_token)
        .env("VAULT_TOKEN", "root-token")
        .assert()
        .success()
        .stdout(contains("Successfully rotated all secrets"));

    let json = read_secret_as_json(http_client, vault_url.as_str()).await;

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

    // Expect connection works; password has been changed
    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user1",
        json["data"]["data"]["postgresql_user_1_password"]
            .as_str()
            .unwrap(),
    )
    .await;

    // Expect connection works; password has been changed
    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user2",
        json["data"]["data"]["postgresql_user_2_password"]
            .as_str()
            .unwrap(),
    )
    .await;

    // Kill `kubectl port-forward` process
    let _ = port_forward
        .kill()
        .expect("Failed to stop port forward-process");

    delete_argocd_deployment();
}

#[tokio::test]
#[timeout(30_000)]
async fn rotate_invalid_initialized_secret() {
    let vault_container = common::vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    let http_client = Client::new();

    let vault_url = format!(
        "http://{vault_host}:{vault_port}/v1/secret/data/rotate/invalid/initialized/secret"
    );
    create_invalid_vault_secret_path(&http_client, vault_url.as_str()).await;

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
argo_cd:
  application: 'propeller'
  base_url: 'http://localhost:8080'
postgres:
  host: 'localhost'
  port: 5432
  database: 'demo'
vault:
  base_url: 'http://{vault_host}:{vault_port}'
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

#[tokio::test]
#[timeout(30_000)]
async fn rotate_non_existing_secret() {
    let vault_container = common::vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    Command::new(&*BIN_PATH)
        .arg("rotate")
        .arg("-c")
        .arg(common::write_string_to_tempfile(
            format!(
                // language=yaml
                "
argo_cd:
  application: 'propeller'
  base_url: 'http://localhost:8080'
postgres:
  host: 'localhost'
  port: 5432
  database: 'demo'
vault:
  base_url: 'http://{vault_host}:{vault_port}'
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

fn deploy_argocd() {
    let kubectl_apply = Command::new("kubectl")
        .args(&["apply", "-f", "argo-cd/manifests/install.yaml"])
        .output()
        .expect("Cannot run 'kubectl apply'");
    if !kubectl_apply.status.success() {
        let error = String::from_utf8_lossy(&kubectl_apply.stderr);
        panic!("Cannot run 'kubectl apply': {}", error);
    }

    let kubectl_wait = Command::new("kubectl")
        .args(&[
            "wait",
            "--for=condition=Ready",
            "--selector=app.kubernetes.io/name=argocd-server",
            "--timeout=60s", // Especially in GitHub Actions this can take a little longer
            "pod",
        ])
        .output()
        .expect("Failed to wait for ArgoCD; readiness not reached");
    if !kubectl_wait.status.success() {
        let error = String::from_utf8_lossy(&kubectl_wait.stderr);
        panic!(
            "Failed to wait for ArgoCD; readiness not reached': {}",
            error
        );
    }
}

async fn reset_vault_secret_path(client: &Client, url: &str) {
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

    write_vault_secret(client, url, &initial_secret).await
}

async fn create_invalid_vault_secret_path(client: &Client, url: &str) {
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

    write_vault_secret(client, url, &initial_secret).await
}

async fn write_vault_secret(client: &Client, url: &str, initial_secret: &VaultSecretDTO) {
    let status = client
        .post(url)
        .header("X-Vault-Token", "root-token")
        .json(&initial_secret)
        .send()
        .await
        .expect("Failed to write Vault secret to path")
        .status()
        .is_success();
    assert_eq!(status, true, "Failed to write Vault secret")
}

async fn connect_postgres_client(
    host: &str,
    port: &str,
    user: &str,
    password: &str,
) -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(
        format!("host={host} port={port} dbname=demo user={user} password={password}").as_str(),
        NoTls,
    )
    .await
    .expect("Failed to build PostgreSQL connection");

    // The connection object performs the actual communication with the database, so spawn it off to run on its own
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            panic!("Failed to connect to to PostgreSQL: {}", e);
        }
    });

    client
}

async fn reset_role_initial_password(postgres_client: &mut tokio_postgres::Client, role: &str) {
    match postgres_client
        .execute(
            format!("CREATE USER {role} WITH PASSWORD 'initialpw'").as_str(),
            &[],
        )
        .await
    {
        Ok(_) => {}
        Err(_) => {
            postgres_client
                .execute(
                    format!("ALTER ROLE {role} WITH PASSWORD 'initialpw'").as_str(),
                    &[],
                )
                .await
                .expect(format!("Failed to reset '{role}'").as_str());
        }
    }
}

fn open_argocd_server_port_forward() -> (u16, Child) {
    // Start the kubectl port-forward command
    let mut child = Command::new("kubectl")
        .args(&["port-forward", "service/argocd-server", ":80"])
        .stdout(Stdio::piped()) // Capture standard output
        .stderr(Stdio::piped()) // Capture standard error
        .spawn()
        .expect("Failed to deploy ArgoCD");

    // Create a reader for the child's output
    let stdout = child
        .stdout
        .take()
        .expect("Failed to capture standard output");
    let reader = BufReader::new(stdout);

    // Find the random port from the output; example:
    //  $ k port-forward service/argocd-server :443
    //  Forwarding from 127.0.0.1:51246 -> 8080
    //  Forwarding from [::1]:51246 -> 8080
    let mut mapped_port = None;
    for line in reader.lines() {
        let line = line.expect("Failed to read line from stdout");
        if line.contains("Forwarding from") {
            // Split the line by spaces and extract the port number
            if let Some(port_str) = line.split_whitespace().nth(2) {
                if let Some(port) = port_str.split(':').nth(1) {
                    mapped_port = Some(port.to_string());
                    break;
                }
            }
        }
    }

    let port = mapped_port.expect("Failed to find mapped ArgoCD port");

    (u16::from_str(port.as_str()).unwrap(), child)
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

async fn get_argocd_access_token(argocd_url: &str) -> String {
    // Run the kubectl command to get the encoded password
    let output = Command::new("kubectl")
        .args(&[
            "get",
            "secret",
            "argocd-initial-admin-secret",
            "-o",
            "jsonpath={.data.password}",
        ])
        .output()
        .expect("Failed to read initial ArgoCD password");

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to read initial ArgoCD password: {}", error);
    }

    // Get the encoded password from the command output
    let encoded_password = String::from_utf8(output.stdout)
        .expect("Failed to read encoded ArgoCD password from stdout");

    // Decode the base64-encoded password
    let decoded_password = general_purpose::STANDARD
        .decode(encoded_password)
        .expect("Failed to decode initial ArgoCD password");
    let password = String::from_utf8(decoded_password)
        .expect("Failed to read decoded initial ArgoCD password");

    // Create a custom http client that accepts self-signed ArgoCD certificate
    let insecure_client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build custom http client for insecure ArgoCD connection");

    let url = format!("{}/api/v1/session", argocd_url);
    let payload = json!({
        "username": "admin",
        "password": password
    });

    let response = insecure_client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .expect("Failed to exchange ArgoCD token");

    if response.status().is_success() {
        let login_response: LoginResponse = response
            .json()
            .await
            .expect("Failed to read session response");
        login_response.token
    } else {
        let error_message = response
            .text()
            .await
            .expect("Failed to read response message");
        panic!("Failed to get Argo CD token: {}", error_message)
    }
}

async fn create_argocd_application(argocd_url: &str, auth_token: &str) {
    // Create a custom http client that accepts self-signed ArgoCD certificate
    let insecure_client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build custom http client for insecure ArgoCD connection");

    let url = format!("{}/api/v1/applications", argocd_url);

    let payload = json!({
        "metadata": {
            "name": "propeller"
        },
        "spec": {
            "project": "default",
            "source": {
                // TODO: Once made public, change URL
                // "repoURL": "https://github.com/postfinance/propeller.git",
                "repoURL": "https://github.com/bbortt/propeller-deployment.git",
                "path": "dev/argo-cd",
                "targetRevision": "main"
            },
            "destination": {
                "server": "https://kubernetes.default.svc",
                "namespace": "default"
            },
            "syncPolicy": {
                "automated": {
                    "prune": true,
                    "selfHeal": true
                }
            }
        }
    });

    let response = insecure_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await
        .expect("Failed to create application in ArgoCD");

    if !response.status().is_success() {
        let error_message = response
            .text()
            .await
            .expect("Failed to read response message");
        panic!("Failed to create application in ArgoCD: {}", error_message)
    }
}

async fn read_secret_as_json(http_client: Client, url: &str) -> Value {
    let response: Response = http_client
        .get(url)
        .header("X-Vault-Token", "root-token")
        .send()
        .await
        .expect("Failed to receive Vault data");

    response
        .error_for_status_ref()
        .expect("Failed to reach Vault");

    let json: Value = response
        .json()
        .await
        .expect("Failed to convert Vault response to JSON");
    json
}

fn delete_argocd_deployment() {
    let kubectl_delete = Command::new("kubectl")
        .args(&["delete", "-f", "argo-cd/manifests/install.yaml"])
        .output()
        .expect("Cannot run 'kubectl delete'");
    if !kubectl_delete.status.success() {
        let error = String::from_utf8_lossy(&kubectl_delete.stderr);
        panic!("Cannot run 'kubectl delete': {}", error);
    }
}
