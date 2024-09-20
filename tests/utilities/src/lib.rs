// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod, Secret};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{ApiResource, DynamicObject, GroupVersionKind, ListParams, Patch, PatchParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::discovery::{ApiCapabilities, Scope};
use kube::runtime::conditions::is_pod_running;
use kube::runtime::wait::await_condition;
use kube::{Api, Client, Config, Discovery, ResourceExt};
use rand::random;
use rustls::crypto::CryptoProvider;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_yaml::from_value;
use std::collections::BTreeMap;
use std::env::temp_dir;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use testcontainers_modules::hashicorp_vault::HashicorpVault;
use testcontainers_modules::k3s::{K3s, KUBE_SECURE_PORT};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};
use tokio::{join, select, spawn};
use tokio_stream::wrappers::TcpListenerStream;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

const ARGOCD_NAMESPACE: &'static str = "argocd";
const ARGOCD_SERVER_LABEL_SELECTOR: &'static str = "app.kubernetes.io/name=argocd-server";

pub async fn postgres_container() -> ContainerAsync<Postgres> {
    Postgres::default()
        .with_env_var("POSTGRES_DB", "demo")
        .with_env_var("POSTGRES_USER", "demo")
        .with_env_var("POSTGRES_PASSWORD", "demo_password")
        .with_userns_mode("host")
        .start()
        .await
        .expect("Failed to launch PostgreSQL database")
}

pub async fn vault_container() -> ContainerAsync<HashicorpVault> {
    HashicorpVault::default()
        .with_env_var("VAULT_DEV_ROOT_TOKEN_ID", "root-token")
        .with_userns_mode("host")
        .start()
        .await
        .expect("Failed to launch Vault")
}

pub async fn k3s_container() -> ContainerAsync<K3s> {
    let conf_dir = temp_dir();
    K3s::default()
        .with_conf_mount(&conf_dir)
        .with_privileged(true)
        .with_userns_mode("host")
        // See: https://github.com/kube-rs/kube/blob/c9753350a5ef4ed22204055d023bc650c07a8629/.github/workflows/ci.yml#L165
        .with_cmd(
            [
                "server",
                "--disable=metrics-server@server*:,servicelb,traefik",
                "--disable-helm-controller",
            ]
            .into_iter(),
        )
        .start()
        .await
        .expect("Failed to launch k3s")
}

pub async fn get_kube_client(container: &ContainerAsync<K3s>) -> Client {
    if CryptoProvider::get_default().is_none() {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Error initializing rustls provider");
    }

    let conf_yaml = container
        .image()
        .read_kube_config()
        .expect("Failed to read kube config from pod");

    // Use this if you want to connect `kubectl` to k3s
    //println!("conf_yaml: {}", conf_yaml);

    let mut config = Kubeconfig::from_yaml(&conf_yaml).expect("Failed to parse kube config yaml");

    let port = container
        .get_host_port_ipv4(KUBE_SECURE_PORT)
        .await
        .expect("Failed to read kube secure port");
    config.clusters.iter_mut().for_each(|cluster| {
        if let Some(server) = cluster.cluster.as_mut().and_then(|c| c.server.as_mut()) {
            *server = format!("https://127.0.0.1:{port}")
        }
    });

    let client_config = Config::from_custom_kubeconfig(config, &KubeConfigOptions::default())
        .await
        .expect("Failed to create client config from kube config");

    kube::Client::try_from(client_config).expect("Failed to create client from client config")
}

pub async fn deploy_argocd_and_wait_until_ready(kubectl: &Client) {
    // k3s exposes discovery when ready, therefore waiting for it with a timeout
    let iteration_duration = Duration::from_secs(3);
    let timeout_duration = Duration::from_secs(60);

    let k3s_discovery = get_discovery(kubectl, iteration_duration, timeout_duration).await;

    join!(
        create_namespace(kubectl, ARGOCD_NAMESPACE),
        create_namespace(kubectl, "propeller")
    );

    let patch_params = PatchParams::apply("kubectl-light").force();
    let yaml_content =
        read_to_string("argo-cd/manifests/install.yaml").expect("Failed to read ArgoCD manifest");

    // See https://github.com/kube-rs/kube/blob/4f78137271361008e5c600582e9c683ddea0d08e/examples/kubectl.rs#L156
    for doc in multidoc_deserialize(&yaml_content).expect("Failed parsing ArgoCD manifest") {
        let obj: DynamicObject =
            from_value(doc).expect("Failed to create dynamic k3s object from ArgoCD manifest");
        let namespace = obj.metadata.namespace.as_deref().or(Some(ARGOCD_NAMESPACE));
        let gvk = if let Some(tm) = &obj.types {
            GroupVersionKind::try_from(tm).expect("Failed to extract GVK")
        } else {
            panic!("Failed to apply object without valid TypeMeta {:?}", obj);
        };

        let name = obj.name_any();

        // Note: Filtering applied resources helps reduce limited resource usage in k3s
        if name.contains("dex") || name.contains("metrics") {
            continue;
        }

        if let Some((ar, caps)) = k3s_discovery.resolve_gvk(&gvk) {
            let api = dynamic_api(ar, caps, kubectl.clone(), namespace, false);
            let data: Value = serde_json::to_value(&obj).expect("Failed");
            api.patch(&name, &patch_params, &Patch::Apply(data))
                .await
                .expect("Failed to apply object to k3s");
        } else {
            panic!("Failed to apply document for unknown {:?}", gvk);
        }
    }

    let pods: Api<Pod> = Api::namespaced(kubectl.clone(), ARGOCD_NAMESPACE);

    join!(
        await_pod_is_running(&pods, ARGOCD_SERVER_LABEL_SELECTOR, timeout_duration),
        await_pod_is_running(
            &pods,
            "app.kubernetes.io/name=argocd-repo-server",
            timeout_duration,
        )
    );
}

async fn get_discovery(
    kubectl: &Client,
    iteration_duration: Duration,
    timeout_duration: Duration,
) -> Discovery {
    let k3s_discovery: Discovery;
    let start_time = Instant::now();

    loop {
        match Discovery::new(kubectl.clone()).run().await {
            Ok(discovery) => {
                k3s_discovery = discovery;
                break;
            }
            Err(e) => {
                if start_time.elapsed() >= timeout_duration {
                    panic!(
                        "Discovery failed after {} seconds: {}",
                        timeout_duration.as_secs(),
                        e
                    );
                }
                sleep(iteration_duration).await;
            }
        };
    }
    k3s_discovery
}

async fn create_namespace(kubectl: &Client, namespace_name: &str) {
    let namespaces: Api<Namespace> = Api::all(kubectl.clone());

    let argocd_namespace = Namespace {
        metadata: ObjectMeta {
            name: Some(namespace_name.to_string()),
            labels: create_singleton_btree("name", namespace_name),
            ..Default::default()
        },
        ..Default::default()
    };

    namespaces
        .create(&Default::default(), &argocd_namespace)
        .await
        .expect(format!("Failed to create namespace '{namespace_name}'").as_str());
}

fn create_singleton_btree(key: &str, value: &str) -> Option<BTreeMap<String, String>> {
    let mut data = BTreeMap::new();
    data.insert(key.to_string(), value.to_string());
    Some(data)
}

fn multidoc_deserialize(
    data: &str,
) -> Result<Vec<serde_yaml::Value>, Box<dyn std::error::Error + 'static>> {
    use serde::Deserialize;
    let mut docs = vec![];
    for de in serde_yaml::Deserializer::from_str(data) {
        docs.push(serde_yaml::Value::deserialize(de)?);
    }
    Ok(docs)
}

fn dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    ns: Option<&str>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, &ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, &ar)
    } else {
        Api::default_namespaced_with(client, &ar)
    }
}

async fn await_pod_is_running(pods: &Api<Pod>, label_filter: &str, timeout_duration: Duration) {
    let pod_name = get_pod_name_matching_label_filter(&pods, label_filter).await;
    let _ = timeout(
        timeout_duration,
        await_condition(pods.clone(), pod_name.as_str(), is_pod_running()),
    )
    .await
    .expect(format!("Timed out waiting for pod matching filter: {label_filter}",).as_str());
}

async fn get_pod_name_matching_label_filter(pods: &Api<Pod>, label_filter: &str) -> String {
    let mut argocd_server: Option<String> = None;
    let list_params = ListParams::default().labels(label_filter);
    for pod in pods
        .list(&list_params)
        .await
        .expect("Failed to find 'argocd-server' pod")
    {
        argocd_server = Some(pod.name_any());
    }
    argocd_server.expect("Failed to find 'argocd-server' pod")
}

pub async fn open_argocd_server_port_forward(kubectl: &Client) -> (u16, oneshot::Sender<()>) {
    let pods: Api<Pod> = Api::namespaced(kubectl.clone(), ARGOCD_NAMESPACE);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to find free random port");
    let bound_port = listener
        .local_addr()
        .expect("Failed to unwrap TCP listener address")
        .port();

    let (stop_sender, stop_receiver) = oneshot::channel::<()>();
    let is_running = Arc::new(AtomicBool::new(true));

    let pod_name = get_pod_name_matching_label_filter(&pods, ARGOCD_SERVER_LABEL_SELECTOR).await;

    let port_forward = async move {
        let mut incoming = TcpListenerStream::new(listener);

        select! {
            _ = stop_receiver => {
                is_running.store(false, Ordering::SeqCst);
            }
            _ = async {
                while is_running.load(Ordering::SeqCst) {
                    if let Some(Ok(client_conn)) = incoming.next().await {
                        let pods = pods.clone();
                        let pod_name_ref = pod_name.clone();

                        spawn(async move {
                            if let Err(e) = forward_connection(&pods, pod_name_ref.as_str(), 8080, client_conn).await {
                                panic!("Failed to forward connection: {}", e);
                            }
                        });
                    }
                }
            } => {}
        }
    };

    // Spawn the port-forwarding process off to run on its own
    spawn(port_forward);

    (bound_port, stop_sender)
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder
        .take_stream(port)
        .expect("Failed to find pod in forwarder");
    tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
    drop(upstream_conn);
    forwarder.join().await?;
    Ok(())
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

pub async fn get_argocd_access_token(kubectl: &Client, argocd_url: &str) -> String {
    let secrets: Api<Secret> = Api::namespaced(kubectl.clone(), ARGOCD_NAMESPACE);

    let secret = secrets
        .get("argocd-initial-admin-secret")
        .await
        .expect("Failed to fetch ArgoCD initial admin secret");
    let password_data = secret
        .data
        .and_then(|mut data| data.remove("password"))
        .expect("Failed to react password from initial admin secret");

    let password = match String::from_utf8(password_data.0.clone()) {
        Ok(password) => password,
        Err(_) => {
            let decoded = BASE64_STANDARD
                .decode(password_data.0)
                .expect("Failed to decode base64 string");
            String::from_utf8(decoded).expect("Failed to extract initial admin password")
        }
    };

    // Create a custom http client that accepts self-signed ArgoCD certificate
    let insecure_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build custom http client for insecure ArgoCD connection");

    let url = format!("{argocd_url}/api/v1/session");
    let authentication_information = json!({
        "username": "admin",
        "password": password
    });

    let response = insecure_client
        .post(&url)
        .json(&authentication_information)
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

pub fn create_vault_client(vault_host: &str, vault_port: u16) -> VaultClient {
    VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(format!("http://{vault_host}:{vault_port}"))
            .token("root-token")
            .build()
            .expect("Failed to build vault client settings"),
    )
    .expect("Failed to build vault client")
}

#[derive(Deserialize, Serialize)]
pub struct VaultSecret {
    pub postgresql_active_user: String,
    pub postgresql_active_user_password: String,
    pub postgresql_user_1: String,
    pub postgresql_user_1_password: String,
    pub postgresql_user_2: String,
    pub postgresql_user_2_password: String,
}

pub async fn read_vault_secret(vault_client: &VaultClient, secret_path: &str) -> VaultSecret {
    from_value(
        kv2::read(vault_client, "secret", secret_path)
            .await
            .expect("Failed to read Vault secret"),
    )
    .expect("Failed to parse Vault secret")
}

pub fn write_string_to_tempfile(content: &str) -> String {
    let mut dir = temp_dir();
    let filename = format!("temp_file_{suffix}", suffix = random::<u64>());

    dir.push(filename);

    let mut file = File::create(dir.clone()).expect("Failed to create tmp file");

    file.write_all(content.as_bytes())
        .expect("Failed to write into tmp file");

    dir.to_string_lossy().to_string()
}
