use std::collections::HashMap;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::{Client as HttpClient, Client};
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::CLI_ARGS;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct ArgoCDConfig {
    api_url: String,
    namespace: String,
    token: String,
}

impl ArgoCDConfig {
    pub fn new(api_url: &str, namespace: &str, token: &str) -> Self {
        ArgoCDConfig {
            api_url: api_url.to_string(),
            namespace: namespace.to_string(),
            token: token.to_string(),
        }
    }
}

pub(crate) struct ArgoCDClient {
    api_url: String,
    client: HttpClient,
    headers: HashMap<String, String>,
    namespace: String,
}

impl ArgoCDClient {
    pub(crate) fn new(argo_cd_config: &ArgoCDConfig) -> Self {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", argo_cd_config.token)
                .parse()
                .expect("🛑 Failed to configure ArgoCD Authentication!"),
        );
        headers.insert(
            "Content-Type".to_string(),
            "application/json"
                .parse()
                .expect("🛑 Failed to configure the ArgoCD client!"),
        );

        ArgoCDClient {
            api_url: argo_cd_config.api_url.to_string(),
            client: HttpClient::new(),
            headers,
            namespace: argo_cd_config.namespace.to_string(),
        }
    }

    pub(crate) fn rollout_namespace(&mut self) {
        if CLI_ARGS.dry_run {
            println!("🧪 The ArgoCD synchronization would now start");
            return;
        } else {
            println!("✅ Perform ArgoCD rollout")
        }

        sync_namespace(
            self.api_url.as_str(),
            self.namespace.as_str(),
            &self.client,
            HeaderMap::try_from(&self.headers).expect("🛑 An unexpected error occurred while creating the ArgoCD synchronization request!"),
        );

        wait_for_rollout(self.api_url.as_str(), self.namespace.as_str(), &self.client);

        println!(
            "✅ Secret change successfully rolled out to namespace '{}'",
            self.namespace
        )
    }
}

fn sync_namespace(api_url: &str, namespace: &str, client: &Client, headers: HeaderMap) {
    let sync_endpoint = format!("{}/api/v1/applications/{}/sync", api_url, namespace);

    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Synchronizing ArgoCD namespace: {}", sync_endpoint);
    }

    let response = client.post(&sync_endpoint).headers(headers).send().expect(
        format!(
            "🛑 Failed to trigger the synchronization of namespace '{}'!",
            namespace
        )
        .as_str(),
    );

    match response.status() {
        StatusCode::OK => {
            if CLI_ARGS.verbose {
                println!(
                    "👀 Namespace '{}' synchronization successfully triggered",
                    namespace
                )
            }
        }
        _ => {
            eprintln!("🛑 Failed to synchronize namespace '{}'", namespace);
            exit(1);
        }
    }
}

#[derive(Debug, Deserialize)]
struct ArgoCDApplicationsResponseStatusHealth {
    status: String,
}

#[derive(Debug, Deserialize)]
struct ArgoCDApplicationsResponseStatus {
    health: ArgoCDApplicationsResponseStatusHealth,
}

#[derive(Debug, Deserialize)]
struct ArgoCDApplicationResponse {
    status: ArgoCDApplicationsResponseStatus,
}

fn wait_for_rollout(api_url: &str, namespace: &str, client: &Client) {
    let rollout_endpoint = format!("{}/api/v1/applications/{}", api_url, namespace);

    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Wait for ArgoCD rollout '{}' to complete", namespace);
    }

    // TODO: Make configurable
    let timeout = Duration::from_secs(60);
    // TODO: Make configurable
    let sleep_time = Duration::from_secs(5);
    let start_time = Instant::now();

    if CLI_ARGS.verbose {
        println!(
            "👀 Timeout: {:?} s, time in between retries: {:?} s",
            timeout, sleep_time
        )
    }

    loop {
        if Instant::now().duration_since(start_time) >= timeout {
            eprintln!(
                "🛑 Timeout reached waiting for rollout of namespace '{}' to finish",
                namespace
            );
            exit(1);
        }

        let response = client.get(&rollout_endpoint).send().expect(
            format!(
                "🛑 Failed to wait for rollout of namespace '{}'!",
                namespace
            )
            .as_str(),
        );

        if response.status() != StatusCode::OK {
            eprintln!("🛑 Failed to request rollout status of namespace '{}' - server returned http status {}", namespace, response.status());
            thread::sleep(sleep_time);
            continue;
        }

        let rollout_response: ArgoCDApplicationResponse = response.json().expect("🛑 ArgoCD returned an unexpected non-JSON body when requesting application information!");
        let rollout_status = &rollout_response.status.health.status;

        if CLI_ARGS.debug || CLI_ARGS.verbose {
            println!(
                "🔎 Checking status of rollout '{}' in namespace '{}'",
                rollout_status, namespace
            );
        }

        if rollout_status != "Healthy" {
            break;
        } else {
            if CLI_ARGS.verbose {
                println!("👀 Rollout was '{}' - not healthy!", rollout_status)
            }

            thread::sleep(sleep_time);
        }
    }
}
