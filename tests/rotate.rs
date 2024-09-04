extern crate core;

use assert_cmd::prelude::*;
use ntest::timeout;
use postgres::NoTls;
use predicates::str::contains;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use utilities::{
    create_vault_client, deploy_argocd_and_wait_until_ready, get_argocd_access_token,
    get_kube_client, k3s_container, open_argocd_server_port_forward, postgres_container,
    read_secret_as_json, vault_container, write_string_to_tempfile,
};
use vaultrs::client::VaultClient;
use vaultrs::kv2;

#[tokio::test]
#[timeout(300_000)]
async fn rotate_secrets() {
    let k3s_container = k3s_container().await;
    let kubectl = get_kube_client(&k3s_container).await;
    deploy_argocd_and_wait_until_ready(&kubectl).await;

    let postgres_container = postgres_container().await;

    let postgres_host = postgres_container.get_host().await.unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .await
        .unwrap()
        .to_string();

    let vault_container = vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    let vault_client = create_vault_client(vault_host.to_string().as_str(), vault_port);
    reset_vault_secret_path(&vault_client, "rotate/secrets").await;

    let mut postgres_client = connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "demo",
        "demo_password",
    )
    .await;

    reset_role_initial_password(&mut postgres_client, "user1").await;
    reset_role_initial_password(&mut postgres_client, "user2").await;

    let (argocd_port, server_future, stop_sender) = open_argocd_server_port_forward(&kubectl).await;
    tokio::spawn(server_future);

    let argocd_url = format!("http://localhost:{}", argocd_port);

    let argocd_token = get_argocd_access_token(&kubectl, argocd_url.as_str()).await;
    create_argocd_application(argocd_url.as_str(), argocd_token.as_str()).await;

    println!("Setup success; invoking propeller...");

    let mut child = Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("-c")
        .arg(write_string_to_tempfile(
            format!(
                // language=yaml
                "
    argo_cd:
      application: 'propeller'
      base_url: 'http://127.0.0.1:{argocd_port}'
      danger_accept_insecure: true
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
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        // .assert()
        // .success()
        // .stdout(contains("Successfully rotated all secrets"));
        .spawn()
        .expect("Failed to start command");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn a task to print stdout
    tokio::spawn(async move {
        for line in stdout_reader.lines() {
            println!("STDOUT: {}", line.expect("Failed to read line"));
        }
    });

    // Spawn a task to print stderr
    tokio::spawn(async move {
        for line in stderr_reader.lines() {
            println!("STDERR: {}", line.expect("Failed to read line"));
        }
    });

    // Wait for the command to complete
    let status = child.wait().expect("Failed to wait for command");
    assert!(status.success());

    let json = read_secret_as_json(&vault_client, "rotate/secrets").await;

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
    stop_sender.send(()).expect("Failed to send stop signal");
}

#[tokio::test]
#[timeout(300_000)]
async fn rotate_application_sync_timeout() {
    let k3s_container = k3s_container().await;
    let kubectl = get_kube_client(&k3s_container).await;
    deploy_argocd_and_wait_until_ready(&kubectl).await;

    let postgres_container = postgres_container().await;

    let postgres_host = postgres_container.get_host().await.unwrap().to_string();
    let postgres_port = postgres_container
        .get_host_port_ipv4(5432)
        .await
        .unwrap()
        .to_string();

    let vault_container = vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    let vault_client = create_vault_client(vault_host.to_string().as_str(), vault_port);
    reset_vault_secret_path(&vault_client, "rotate/secrets/timeout").await;

    let mut postgres_client = connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "demo",
        "demo_password",
    )
    .await;

    reset_role_initial_password(&mut postgres_client, "user1").await;
    reset_role_initial_password(&mut postgres_client, "user2").await;

    let (argocd_port, server_future, stop_sender) = open_argocd_server_port_forward(&kubectl).await;
    tokio::spawn(server_future);

    let argocd_url = format!("http://localhost:{}", argocd_port);

    let argocd_token = get_argocd_access_token(&kubectl, argocd_url.as_str()).await;
    create_argocd_application(argocd_url.as_str(), argocd_token.as_str()).await;

    println!("Setup success; invoking propeller...");

    Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("-c")
        .arg(write_string_to_tempfile(
            format!(
                // language=yaml
                "
    argo_cd:
      application: 'propeller'
      base_url: 'http://127.0.0.1:{argocd_port}'
      danger_accept_insecure: true
      sync_timeout_seconds: 5
    postgres:
      host: '{postgres_host}'
      port: {postgres_port}
      database: 'demo'
    vault:
      base_url: 'http://{vault_host}:{vault_port}'
      path: 'rotate/secrets/timeout'
"
            )
            .as_str(),
        ))
        .env("ARGO_CD_TOKEN", argocd_token)
        .env("VAULT_TOKEN", "root-token")
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains(
            // The configured sync timeout of 5 seconds is no match for the 10 seconds sleep in the pre-sync hook
            "Timeout reached while waiting for ArgoCD rollout to complete",
        ));

    // Kill `kubectl port-forward` process
    stop_sender.send(()).expect("Failed to send stop signal");
}

#[tokio::test]
#[timeout(30_000)]
async fn rotate_missing_vault_token() {
    Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("-c")
        .arg(write_string_to_tempfile(
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
  base_url: 'http://localhost:8200'
  path: 'rotate/non/existing/path'
"
            )
            .as_str(),
        ))
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains("Missing VAULT_TOKEN environment variable"));
}

#[tokio::test]
#[timeout(30_000)]
async fn rotate_invalid_initialized_secret() {
    let vault_container = vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    let vault_client = create_vault_client(vault_host.to_string().as_str(), vault_port);
    create_invalid_vault_secret_path(&vault_client, "rotate/invalid/initialized/secret").await;

    println!("Setup success; invoking propeller...");

    Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("-c")
        .arg(write_string_to_tempfile(
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
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains(
            "Failed to detect active user - did neither match user 1 nor 2",
        ));
}

#[tokio::test]
#[timeout(30_000)]
async fn rotate_non_existing_secret() {
    let vault_container = vault_container().await;

    let vault_host = vault_container.get_host().await.unwrap();
    let vault_port = vault_container.get_host_port_ipv4(8200).await.unwrap();

    println!("Setup success; invoking propeller...");

    Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("-c")
        .arg(write_string_to_tempfile(
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
        .env("PROPELLER_LOG_LEVEL", "info")
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains(
            "Failed to read path 'rotate/non/existing/path' - did you init Vault?",
        ));
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

async fn reset_vault_secret_path(vault_client: &VaultClient, secret_path: &str) {
    let initial_secret = VaultSecret {
        postgresql_active_user: "user1".to_string(),
        postgresql_active_user_password: "initialpw".to_string(),
        postgresql_user_1: "user1".to_string(),
        postgresql_user_1_password: "initialpw".to_string(),
        postgresql_user_2: "user2".to_string(),
        postgresql_user_2_password: "initialpw".to_string(),
    };

    kv2::set(vault_client, "secret", secret_path, &initial_secret)
        .await
        .expect("Failed to reset Vault secret path");
}

async fn create_invalid_vault_secret_path(vault_client: &VaultClient, secret_path: &str) {
    let initial_secret = VaultSecret {
        postgresql_active_user: "userX".to_string(), // Note that 'userX' does neither match 'user1' nor 'user2'
        postgresql_active_user_password: "initialpw".to_string(),
        postgresql_user_1: "user1".to_string(),
        postgresql_user_1_password: "initialpw".to_string(),
        postgresql_user_2: "user2".to_string(),
        postgresql_user_2_password: "initialpw".to_string(),
    };

    kv2::set(vault_client, "secret", secret_path, &initial_secret)
        .await
        .expect("Failed to reset Vault secret path");
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

async fn create_argocd_application(argocd_url: &str, auth_token: &str) {
    // Create a custom http client that accepts self-signed ArgoCD certificate
    let insecure_client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build custom http client for insecure ArgoCD connection");

    let url = format!("{}/api/v1/applications", argocd_url);

    let argocd_application = json!({
        "metadata": {
            "name": "propeller"
        },
        "spec": {
            "project": "default",
            "source": {
                // TODO: Once made public, change URL
                // "repoURL": "https://github.com/postfinance/propeller",
                "repoURL": "https://github.com/bbortt/propeller-deployment",
                "path": "dev/argo-cd",
                "targetRevision": "main"
            },
            "destination": {
                "server": "https://kubernetes.default.svc",
                "namespace": "propeller"
            },
            "syncPolicy": {
                "automated": {
                    "prune": true,
                    "selfHeal": true
                }
            }
        }
    });

    // Pods are "running", but not necessarily "ready" - need another retry here
    let iteration_duration = Duration::from_secs(3);
    let timeout_duration = Duration::from_secs(60);

    let start_time = Instant::now();

    loop {
        let response = insecure_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&argocd_application)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => return,
            Ok(_) => {}
            Err(_) => {}
        }

        if start_time.elapsed() >= timeout_duration {
            panic!(
                "Failed to deploy application to ArgoCD after {} seconds",
                timeout_duration.as_secs(),
            );
        }

        sleep(iteration_duration).await;
    }
}
