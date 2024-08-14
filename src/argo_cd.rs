use log::{debug, info, warn};
use reqwest::Client;
use std::env;
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

        let vault_token = env::var(ARGO_CD_TOKEN);
        let request_builder = match vault_token {
            Ok(token) => request_builder.header("Authorization", format!("Bearer {}", token)),
            Err(_) => {
                warn!("You're accessing ArgoCD without authentication (missing {} environment variable)", ARGO_CD_TOKEN);
                request_builder
            }
        };

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
        let timeout_seconds = self.argo_config.sync_timeout_seconds.unwrap_or(60u16);

        info!(
            "Waiting for rollout of ArgoCD application '{}' to finish - timeout is {} seconds",
            self.argo_config.application, timeout_seconds
        );
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

#[cfg(test)]
mod tests {}
