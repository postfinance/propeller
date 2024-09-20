// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use assert_cmd::prelude::*;
use ntest::timeout;
use postgres::NoTls;
use predicates::str::contains;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::{join, spawn};
use utilities::{
    create_vault_client, deploy_argocd_and_wait_until_ready, get_argocd_access_token,
    get_kube_client, k3s_container, open_argocd_server_port_forward, postgres_container,
    read_vault_secret, vault_container, write_string_to_tempfile, VaultSecret,
};
use vaultrs::client::VaultClient;
use vaultrs::kv2;

#[tokio::test(flavor = "multi_thread")]
async fn rotate_secrets() {
    let (k3s_container, postgres_container, vault_container) =
        join!(k3s_container(), postgres_container(), vault_container());

    let kubectl = get_kube_client(&k3s_container).await;

    let argocd_deployment = deploy_argocd_and_wait_until_ready(&kubectl);

    let (postgres_host, postgres_port, vault_host, vault_port) = join!(
        postgres_container.get_host(),
        postgres_container.get_host_port_ipv4(5432),
        vault_container.get_host(),
        vault_container.get_host_port_ipv4(8200)
    );

    let postgres_host = postgres_host.unwrap().to_string();
    let postgres_port = postgres_port.unwrap().to_string();
    let vault_host = vault_host.unwrap().to_string();
    let vault_port = vault_port.unwrap();

    let vault_client = create_vault_client(&vault_host, vault_port);
    let (_, postgres_client) = join!(
        reset_vault_secret_path(&vault_client, "rotate/secrets"),
        connect_postgres_client(&postgres_host, &postgres_port, "demo", "demo_password",)
    );

    join!(
        reset_role_initial_password(&postgres_client, "user1"),
        reset_role_initial_password(&postgres_client, "user2")
    );

    // Ensure ArgoCD is ready before proceeding
    argocd_deployment.await;

    let (argocd_port, stop_sender) = open_argocd_server_port_forward(&kubectl).await;

    let argocd_url = format!("http://localhost:{argocd_port}");

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
      base_url: 'http://localhost:{argocd_port}'
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
        .env("PROPELLER_LOG_LEVEL", "debug,rustify=off")
        .stdout(Stdio::piped())
        .assert()
        .success()
        .stdout(contains("Successfully rotated all secrets"));

    let vault_secret = read_vault_secret(&vault_client, "rotate/secrets").await;

    assert_eq!(vault_secret.postgresql_active_user, "user2");
    assert_ne!(vault_secret.postgresql_active_user_password, "initialpw");
    assert_eq!(vault_secret.postgresql_user_1, "user1");
    assert_ne!(vault_secret.postgresql_user_1_password, "initialpw");
    assert_eq!(vault_secret.postgresql_user_2, "user2");
    assert_ne!(vault_secret.postgresql_user_2_password, "initialpw");

    // Expect connection works; password has been changed
    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user1",
        vault_secret.postgresql_user_1_password.as_str(),
    )
    .await;

    // Expect connection works; password has been changed
    connect_postgres_client(
        postgres_host.as_str(),
        postgres_port.as_str(),
        "user2",
        vault_secret.postgresql_user_2_password.as_str(),
    )
    .await;

    // Kill `kubectl port-forward` process
    stop_sender.send(()).expect("Failed to send stop signal");
}

#[tokio::test(flavor = "multi_thread")]
async fn rotate_application_sync_timeout() {
    let (k3s_container, postgres_container, vault_container) =
        join!(k3s_container(), postgres_container(), vault_container());

    let kubectl = get_kube_client(&k3s_container).await;

    let argocd_deployment = deploy_argocd_and_wait_until_ready(&kubectl);

    let (postgres_host, postgres_port, vault_host, vault_port) = join!(
        postgres_container.get_host(),
        postgres_container.get_host_port_ipv4(5432),
        vault_container.get_host(),
        vault_container.get_host_port_ipv4(8200)
    );

    let postgres_host = postgres_host.unwrap().to_string();
    let postgres_port = postgres_port.unwrap().to_string();
    let vault_host = vault_host.unwrap().to_string();
    let vault_port = vault_port.unwrap();

    let vault_client = create_vault_client(&vault_host, vault_port);
    let (_, postgres_client) = join!(
        reset_vault_secret_path(&vault_client, "rotate/secrets/timeout"),
        connect_postgres_client(&postgres_host, &postgres_port, "demo", "demo_password",)
    );

    join!(
        reset_role_initial_password(&postgres_client, "user1"),
        reset_role_initial_password(&postgres_client, "user2")
    );

    // Ensure ArgoCD is ready before proceeding
    argocd_deployment.await;

    let (argocd_port, stop_sender) = open_argocd_server_port_forward(&kubectl).await;

    let argocd_url = format!("http://localhost:{argocd_port}");

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
      base_url: 'http://localhost:{argocd_port}'
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
            "Timeout reached while waiting for ArgoCD sync status",
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
    spawn(async move {
        if let Err(e) = connection.await {
            panic!("Failed to connect to to PostgreSQL: {}", e);
        }
    });

    client
}

async fn reset_role_initial_password(postgres_client: &tokio_postgres::Client, role: &str) {
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

    let url = format!("{argocd_url}/api/v1/applications");

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
            .header("Authorization", format!("Bearer {auth_token}"))
            .json(&argocd_application)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => break,
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

    wait_for_argocd_application_rollout(argocd_url, &insecure_client, auth_token).await;
}

#[derive(Debug, Deserialize)]
struct Application {
    status: ApplicationStatus,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ApplicationStatus {
    sync: SyncStatus,
    health: HealthStatus,
}

#[derive(Debug, Deserialize)]
struct SyncStatus {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HealthStatus {
    status: Option<String>,
}

async fn wait_for_argocd_application_rollout(
    argocd_url: &str,
    client: &Client,
    argocd_token: &str,
) {
    let url = format!("{argocd_url}/api/v1/applications/propeller",);

    let request = client
        .get(url.as_str())
        .header("Authorization", format!("Bearer {argocd_token}"))
        .build()
        .expect("Failed to build ArgoCD sync status request");

    let timeout_duration = Duration::from_secs(60);
    let start_time = Instant::now();

    loop {
        if start_time.elapsed() >= timeout_duration {
            panic!("Timeout reached while waiting for ArgoCD sync status");
        }

        let response = client
            .execute(
                request
                    .try_clone()
                    .expect("Failed to build ArgoCD sync status request"),
            )
            .await
            .expect("Failed to request ArgoCD sync status");

        if response.status().is_success() {
            let app_information: Application = response
                .json()
                .await
                .expect("Failed to read ArgoCD sync status response");

            if app_information
                .status
                .sync
                .status
                .unwrap_or_else(|| "NONE".to_string())
                == "Synced"
                && app_information
                    .status
                    .health
                    .status
                    .unwrap_or_else(|| "NONE".to_string())
                    == "Healthy"
            {
                return;
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
