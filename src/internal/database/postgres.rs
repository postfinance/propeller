use std::process::exit;

use postgres::{Client, NoTls};

use crate::internal::database::{DatabaseClient, DatabaseConfig};

pub(crate) struct PostgresClient {
    client: Client,
}

fn create_user(username: &str, password: &str, client: &mut Client) {
    match client.execute("CREATE USER $1 WITH PASSWORD '$2'", &[&username, &password]) {
        Ok(res) => println!("User '{}' successfully created", username),
        Err(err) => {
            eprintln!("Failed to create user '{}': {}", username, err);
        }
    }
}

fn grant_role(username: &str, role: &str, client: &mut Client) {
    match client.execute("GRANT $1 TO $2", &[&role, &username]) {
        Ok(res) => println!("Role '{}' successfully granted to '{}'", role, username),
        Err(err) => {
            eprintln!(
                "Failed to grant role '{}' to user '{}': {}",
                role, username, err
            );
        }
    }
}

fn drop_user(username: &str, client: &mut Client) {
    match client.execute("DROP USER $1", &[&username]) {
        Ok(res) => println!("User '{}' successfully dropped", username),
        Err(err) => {
            eprintln!("Failed to drop user '{}': {}", username, err);
        }
    }
}

impl DatabaseClient for PostgresClient {
    fn new(database_config: &DatabaseConfig) -> Self {
        PostgresClient {
            client: match Client::connect(&database_config.url, NoTls) {
                Ok(client) => client,
                Err(err) => {
                    eprintln!("🛑 Failed to load PostgreSQL configuration: {}", err);
                    exit(1)
                }
            },
        }
    }

    fn get_existing_users(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let result = self.client.query(
            "SELECT usename as role_name FROM pg_catalog.pg_user WHERE usename LIKE '$1%'",
            &[&prefix],
        );

        // Example code to query PostgreSQL and retrieve existing users
        let mut existing_users = Vec::new();

        match result {
            Ok(rows) => {
                for row in &rows {
                    let username: String = row.get("username");
                    existing_users.push(username);
                }
            }
            Err(err) => {
                println!(
                    "Failed to retrieve existing users from PostgreSQL database: {}",
                    err
                );
            }
        }

        Ok(existing_users)
    }

    fn create_user_and_assign_role(
        &mut self,
        username: &str,
        password: &str,
        role: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut_client = &mut self.client;

        create_user(username, password, mut_client);
        grant_role(username, role, mut_client);

        Ok(())
    }

    fn drop_users(&mut self, users: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        for user in users {
            drop_user(&user, &mut self.client)
        }

        Ok(())
    }
}
