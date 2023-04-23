use std::collections::HashMap;

use reqwest::blocking::Client as HttpClient;
use reqwest::header::{HeaderMap, HeaderName};

pub(crate) struct ArgoCDConfig {
    api_url: String,
    namespace: String,
    token: String,
}

impl ArgoCDConfig {
    pub fn new(api_url: String, namespace: String, token: String) -> Self {
        ArgoCDConfig {
            api_url,
            namespace,
            token,
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

    pub(crate) fn sync_namespace(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let sync_url = format!(
            "{}/api/v1/applications/{}/sync",
            self.api_url, self.namespace
        );

        self.client
            .post(&sync_url)
            .headers(HeaderMap::try_from(&self.headers)?)
            .send()?;

        Ok(())
    }
}
