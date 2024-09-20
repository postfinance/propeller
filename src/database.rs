// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use postgres::{Client, NoTls};

use crate::config::{Config, PostgresConfig};

pub(crate) struct PostgresClient {
    postgres_config: PostgresConfig,
}

impl PostgresClient {
    pub(crate) fn init(config: &Config) -> PostgresClient {
        PostgresClient {
            postgres_config: config.postgres.clone(),
        }
    }

    pub(crate) fn connect_for_user(&self, username: String, password: String) -> Client {
        let host = self.postgres_config.host.as_str();
        let port = self.postgres_config.port;
        let database = self.postgres_config.database.as_str();

        Client::connect(
            format!(
                "host={host} port={port} dbname={database} user={username} password={password}"
            )
            .as_str(),
            NoTls,
        )
        .expect("Failed to build PostgreSQL connection")
    }
}
