use std::process::exit;

use postgres::{Client, NoTls};

use crate::internal::database::{DatabaseClient, DatabaseConfig};

struct PostgresClient {
    client: Client,
}

impl DatabaseClient for PostgresClient {
    fn new(database_config: &DatabaseConfig) -> Self {
        PostgresClient {
            client: match Client::connect(&database_config.url, NoTls) {
                Ok(client) => client,
                Err(err) => {
                    println!("🛑 Failed to load PostgreSQL configuration: {}", err);
                    exit(1)
                }
            }
        }
    }

    fn get_existing_users(prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // TODO: Implement
        Ok(Vec::new())
    }

    fn create_user(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement
        Ok(())
    }

    fn delete_users_from_postgres(users: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement
        Ok(())
    }
}
