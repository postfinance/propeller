use log::{debug, info};
use reqwest::Client;
use tokio::runtime::{Builder, Runtime};

use crate::config::{ArgoConfig, Config};

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
            client: Client::new(),
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

        let local_var_uri_str = format!(
            "{}/api/v1/applications/{name}/sync",
            self.argo_config.base_url,
            name = urlencode(self.argo_config.application.as_str())
        );
        let mut local_var_req_builder = self
            .client
            .request(reqwest::Method::POST, local_var_uri_str.as_str());

        local_var_req_builder = local_var_req_builder.json("&body"); // TODO

        let local_var_req = local_var_req_builder
            .build()
            .expect("Failed to build ArgoCD sync request");
        let local_var_resp = self
            .rt
            .block_on(self.client.execute(local_var_req))
            .expect("Failed to sync ArgoCD");

        let local_var_status = local_var_resp.status();

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            let local_var_content = self
                .rt
                .block_on(local_var_resp.text())
                .unwrap_or_else(|_| panic!("Failed to sync ArgoCD"));
            panic!("Failed to sync ArgoCD: {}", local_var_content)
        }
    }

    pub(crate) fn wait_for_rollout(&mut self) {
        let timeout_seconds = self
            .argo_config
            .sync_timeout_seconds
            .unwrap_or_else(|| 60u16);

        info!(
            "Waiting for rollout of ArgoCD application '{}' to finish - timeout is {} seconds",
            self.argo_config.application, timeout_seconds
        );
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}
