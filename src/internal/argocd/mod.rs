use std::collections::HashMap;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use hashicorp_vault::url::quirks::username;
use reqwest::blocking::{Client as HttpClient, Client};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

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
            format!("Bearer {}", argo_cd_config.token).parse().unwrap(),
        );
        headers.insert(
            "Content-Type".to_string(),
            "application/json".parse().unwrap(),
        );

        ArgoCDClient {
            api_url: argo_cd_config.api_url.to_string(),
            client: HttpClient::new(),
            headers,
            namespace: argo_cd_config.namespace.to_string(),
        }
    }

    pub(crate) fn rollout_namespace(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        sync_namespace(
            self.api_url.as_str(),
            self.namespace.as_str(),
            &self.client,
            HeaderMap::try_from(&self.headers)?,
        )?;

        wait_for_rollout(self.api_url.as_str(), self.namespace.as_str(), &self.client);

        Ok(())
    }
}

fn sync_namespace(
    api_url: &str,
    namespace: &str,
    client: &Client,
    headers: HeaderMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let sync_endpoint = format!("{}/api/v1/applications/{}/sync", api_url, namespace);

    let response = match client
        .post(&sync_endpoint)
        .headers(HeaderMap::try_from(&headers)?)
        .send()
    {
        Ok(response) => response,
        Err(err) => {
            eprintln!(
                "🛑 Error while triggering synchronisation of namespace '{}': {}",
                namespace, err
            );
            exit(1)
        }
    };

    match response.status() {
        StatusCode::OK => {
            println!(
                "✅ Namespace '{}' synchronisation successfully triggered",
                namespace
            )
        }
        (_) => {
            eprintln!("🛑 Failed to synchronise namespace '{}'", namespace);
            exit(1);
        }
    }

    Ok(())
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

    let timeout = Duration::from_secs(60);
    let sleep_time = Duration::from_secs(5);
    let start_time = Instant::now();

    loop {
        if (Instant::now().duration_since(start_time) >= timeout) {
            eprintln!(
                "🛑 Timeout reached waiting for rollout of namespace '{}' to finish",
                namespace
            );
            exit(1);
        }

        let response = match client.get(&rollout_endpoint).send() {
            Ok(response) => response,
            Err(err) => {
                eprintln!(
                    "🛑 Failed to wait for rollout of namespace '{}': {}",
                    namespace, err
                );
                exit(1)
            }
        };

        if (response.status() != StatusCode::OK) {
            eprintln!("🛑 Failed to request rollout status of namespace '{}' - server returned http status {}", namespace, response.status());
            thread::sleep(sleep_time);
            continue;
        }

        let rollout_response: ArgoCDApplicationResponse = response.json()?;
        let rollout_status = &rollout_response.status.health.status;

        println!(
            "Checking status of rollout '{}' in namespace '{}'",
            rollout_status, namespace
        );

        if (rollout_status == "Healthy") {
            return;
        } else {
            thread::sleep(sleep_time);
        }
    }
}
