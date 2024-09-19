use log::{debug, info, warn};
use reqwest::header::CONTENT_TYPE;
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

        let app_name = self.argo_config.application.as_str();
        let url = format!(
            "{baseUrl}/api/v1/applications/{name}/sync",
            baseUrl = self.argo_config.base_url,
            name = encode(app_name)
        );

        let request_builder = self
            .client
            .post(url.as_str())
            .header(CONTENT_TYPE, "application/json");
        let request_builder = Self::enhance_with_authorization_token_if_applicable(request_builder);

        let request = request_builder
            .build()
            .expect("Failed to build ArgoCD sync request");

        let response = self
            .rt
            .block_on(self.client.execute(request))
            .expect("Failed to sync ArgoCD");

        let response_status = response.status();
        if response_status.is_client_error() || response_status.is_server_error() {
            let argocd_response = self
                .rt
                .block_on(response.text())
                .expect("Failed to sync ArgoCD");
            // TODO: Think if this is problematic;
            // Failed to sync ArgoCD: {"error":"another operation is already in progress","code":9,"message":"another operation is already in progress"}

            panic!("Failed to sync ArgoCD: {}", argocd_response)
        }

        debug!("ArgoCD sync triggered, waiting for status update");

        fn is_status_in_progress(app_information: &Application) -> bool {
            app_information
                .status
                .operationState
                .as_ref()
                .map(|operation_state| operation_state.phase.clone())
                .unwrap_or_else(|| "NONE".to_string())
                == "Running"
        }

        self.wait_for_status_change(is_status_in_progress)
    }

    pub(crate) fn wait_for_rollout(&mut self) {
        info!(
            "Waiting for rollout of ArgoCD application '{}' to finish - timeout is {} seconds",
            self.argo_config.application,
            self.get_sync_timeout_seconds()
        );

        fn is_status_synced(app_information: &Application) -> bool {
            app_information.status.sync.status == "Synced"
                && app_information.status.health.status == "Healthy"
                && (app_information.status.operationState.is_none()
                    || app_information
                        .status
                        .operationState
                        .as_ref()
                        .map(|operation_state| operation_state.phase.clone())
                        .unwrap_or_else(|| "NONE".to_string())
                        == "Succeeded")
        }

        self.wait_for_status_change(is_status_synced)
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

    fn enhance_with_authorization_token_if_applicable(
        request_builder: RequestBuilder,
    ) -> RequestBuilder {
        let argocd_token = env::var(ARGO_CD_TOKEN);
        match argocd_token {
            Ok(token) => {
                debug!("Applying bearer token to ArgoCD request");
                request_builder.header("Authorization", format!("Bearer {token}"))
            }
            Err(_) => {
                warn!("You're accessing ArgoCD without authentication (missing {} environment variable)", ARGO_CD_TOKEN);
                request_builder
            }
        }
    }

    fn get_sync_timeout_seconds(&self) -> u64 {
        match self.argo_config.sync_timeout_seconds {
            Some(seconds) => seconds as u64,
            None => 60,
        }
    }

    fn wait_for_status_change(&mut self, condition: fn(&Application) -> bool) {
        let url = format!(
            "{baseUrl}/api/v1/applications/{name}",
            baseUrl = self.argo_config.base_url,
            name = encode(self.argo_config.application.as_str())
        );

        let request_builder = self.client.get(url.as_str());
        let request_builder = Self::enhance_with_authorization_token_if_applicable(request_builder);

        let request = request_builder
            .build()
            .expect("Failed to build ArgoCD sync status request");

        let timeout_duration = Duration::from_secs(self.get_sync_timeout_seconds());
        let start_time = Instant::now();

        loop {
            if start_time.elapsed() >= timeout_duration {
                panic!("Timeout reached while waiting for ArgoCD sync status");
            }

            let response = self
                .rt
                .block_on(
                    self.client.execute(
                        request
                            .try_clone()
                            .expect("Failed to build ArgoCD sync status request"),
                    ),
                )
                .expect("Failed to request ArgoCD sync status");

            if response.status().is_success() {
                let app_information = match self.rt.block_on(response.json::<Application>()).ok() {
                    Some(app) => app,
                    None => continue,
                };

                debug!("ArgoCD sync status response: {:?}", app_information);

                if condition(&app_information) {
                    info!("Desired ArgoCD application status met");
                    return;
                } else {
                    let status = app_information.status;
                    debug!(
                        "ArgoCD application did not meet desired status yet: {{ 'sync': '{}', 'health': '{}', 'operation_state': '{}' }}",
                        status.sync.status, status.health.status, status.operationState.map(|o| o.phase).unwrap_or_else(|| "None".to_string())
                    );
                }
            } else {
                debug!("Failed to get application status: {}", response.status());
            }

            sleep(Duration::from_secs(5));
        }
    }
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
    operationState: Option<OperationState>,
}

#[derive(Debug, Deserialize)]
struct SyncStatus {
    status: String,
}

#[derive(Debug, Deserialize)]
struct HealthStatus {
    status: String,
}

#[derive(Debug, Deserialize)]
struct OperationState {
    phase: String,
}

#[cfg(test)]
mod tests {}
