// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::config::{Config, PostgresConfig};
use postgres::Error;
use postgres::{Client, NoTls};
use std::sync::Arc;

pub trait ClientFactory {
    fn create_client(&self, connection_string: &str) -> Result<Client, Error>;
}

struct PropellerClientFactory;

impl ClientFactory for PropellerClientFactory {
    fn create_client(&self, connection_string: &str) -> Result<Client, Error> {
        Client::connect(connection_string, NoTls)
    }
}

pub struct PostgresClient {
    postgres_config: PostgresConfig,
    client_factory: Arc<dyn ClientFactory>,
}

impl PostgresClient {
    pub(crate) fn init(config: &Config) -> PostgresClient {
        PostgresClient {
            postgres_config: config.postgres.clone(),
            client_factory: Arc::new(PropellerClientFactory),
        }
    }

    pub(crate) fn connect_for_user(&self, username: String, password: String) -> Client {
        let host = self.postgres_config.host.as_str();
        let port = self.postgres_config.port;
        let database = self.postgres_config.database.as_str();

        let connection_string = format!(
            "host={host} port={port} dbname={database} user={username} password={password}"
        );

        self.client_factory
            .create_client(&connection_string)
            .expect("Failed to build PostgreSQL connection")
    }

    #[cfg(test)]
    pub(crate) fn with_client_factory(
        config: &Config,
        client_factory: Arc<dyn ClientFactory>,
    ) -> PostgresClient {
        PostgresClient {
            postgres_config: config.postgres.clone(),
            client_factory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ArgoConfig, VaultConfig};

    struct MockClientFactory;

    impl ClientFactory for MockClientFactory {
        fn create_client(&self, connection_string: &str) -> Result<Client, Error> {
            assert!(connection_string.contains("host=testhost"));
            assert!(connection_string.contains("port=2345"));
            assert!(connection_string.contains("dbname=testdb"));
            assert!(connection_string.contains("user=testuser"));
            assert!(connection_string.contains("password=testpass"));

            Client::connect("", NoTls)
        }
    }

    #[test]
    fn init() {
        let config = create_config_with_testdb();

        let fixture = PostgresClient::init(&config);

        assert_eq!(fixture.postgres_config.host, "testhost");
        assert_eq!(fixture.postgres_config.port, 2345);
        assert_eq!(fixture.postgres_config.database, "testdb");
    }

    #[test]
    #[should_panic(expected = "both host and hostaddr are missing")]
    fn connect_for_user() {
        let config = create_config_with_testdb();

        let client = PostgresClient::with_client_factory(
            &config,
            Arc::new(MockClientFactory) as Arc<dyn ClientFactory>,
        );

        // This should panic
        client.connect_for_user("testuser".to_string(), "testpass".to_string());
    }

    fn create_config_with_testdb() -> Config {
        Config {
            argo_cd: ArgoConfig::default(),
            postgres: PostgresConfig {
                host: "testhost".to_string(),
                port: 2345,
                database: "testdb".to_string(),
            },
            vault: VaultConfig::default(),
        }
    }
}
