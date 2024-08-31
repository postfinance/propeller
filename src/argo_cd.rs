use log::{debug, info, warn};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};
use urlencoding::encode;

use crate::config::{ArgoConfig, Config};

const ARGO_CD_TOKEN: &str = "ARGO_CD_TOKEN";

pub(crate) struct ArgoCD {
    argo_config: ArgoConfig,
    client: Client,
    rt: Runtime,
}

impl ArgoCD {
    pub(crate) fn init(config: &Config) -> ArgoCD {
        debug!("Connecting to ArgoCD at: {}", config.argo_cd.base_url);

        ArgoCD {
            argo_config: config.argo_cd.clone(),
            client: Self::get_argocd_client(config.argo_cd.clone()),
            rt: Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build ArgoCD connection"),
        }
    }

    pub(crate) fn sync(&mut self) {
        info!(
            "Synchronizing ArgoCD application '{}'",
            self.argo_config.application
        );

        let url = format!(
            "{}/api/v1/applications/{name}/sync",
            self.argo_config.base_url,
            name = encode(self.argo_config.application.as_str())
        );

        let request_builder = self.client.post(url.as_str()).json("&body"); // TODO
        let request_builder = Self::enhance_with_authorization_token_if_applicable(request_builder);

        let request = request_builder
            .build()
            .expect("Failed to build ArgoCD sync request");

        let response = self
            .rt
            .block_on(self.client.execute(request))
            .expect("Failed to sync ArgoCD");

        let response_status = response.status();
        if !response_status.is_client_error() && !response_status.is_server_error() {
            let local_var_content = self
                .rt
                .block_on(response.text())
                .unwrap_or_else(|_| panic!("Failed to sync ArgoCD"));
            panic!("Failed to sync ArgoCD: {}", local_var_content)
        }
    }

    pub(crate) fn wait_for_rollout(&mut self) {
        let timeout_seconds: u64 = match self.argo_config.sync_timeout_seconds {
            Some(seconds) => seconds as u64,
            None => 60,
        };

        info!(
            "Waiting for rollout of ArgoCD application '{}' to finish - timeout is {} seconds",
            self.argo_config.application, timeout_seconds
        );

        let url = format!(
            "{}/api/v1/applications/{name}",
            self.argo_config.base_url,
            name = encode(self.argo_config.application.as_str())
        );

        let request_builder = self.client.get(url.as_str());
        let request_builder = Self::enhance_with_authorization_token_if_applicable(request_builder);

        let request = request_builder
            .build()
            .expect("Failed to build ArgoCD sync status request");

        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(timeout_seconds);

        loop {
            if start_time.elapsed() > timeout_duration {
                panic!("Timeout reached while waiting for ArgoCD rollout to complete");
            }

            let response = self
                .rt
                .block_on(self.client.execute(request.try_clone().unwrap()))
                .expect("Failed to get ArgoCD sync status");

            if response.status().is_success() {
                let app_information: Application = self
                    .rt
                    .block_on(response.json())
                    .expect("Failed to read ArgoCD sync status response");

                if app_information.status.sync.status == "Synced"
                    && app_information.status.health.status == "Healthy"
                {
                    info!("Application rollout completed successfully");
                    return;
                } else {
                    debug!(
                        "Application rollout not finished yet: {{ 'sync': '{}', 'health': '{}' }}",
                        app_information.status.sync.status, app_information.status.health.status
                    );
                }
            } else {
                debug!("Failed to get application status: {}", response.status());
            }

            // Wait for 5 seconds before checking again
            sleep(Duration::from_secs(5));
        }
    }

    fn enhance_with_authorization_token_if_applicable(
        request_builder: RequestBuilder,
    ) -> RequestBuilder {
        let argocd_token = env::var(ARGO_CD_TOKEN);
        match argocd_token {
            Ok(token) => request_builder.header("Authorization", format!("Bearer {}", token)),
            Err(_) => {
                warn!("You're accessing ArgoCD without authentication (missing {} environment variable)", ARGO_CD_TOKEN);
                request_builder
            }
        }
    }

    fn get_argocd_client(argo_config: ArgoConfig) -> Client {
        match argo_config.danger_accept_insecure {
            Some(accept_insecure) => Client::builder()
                .danger_accept_invalid_certs(accept_insecure)
                .build()
                .expect("Failed to build HTTP client"),
            None => Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct Application {
    status: ApplicationStatus,
}

#[derive(Deserialize)]
struct ApplicationStatus {
    sync: SyncStatus,
    health: HealthStatus,
}

#[derive(Deserialize)]
struct SyncStatus {
    status: String,
}

#[derive(Deserialize)]
struct HealthStatus {
    status: String,
}

#[cfg(test)]
mod tests {}
