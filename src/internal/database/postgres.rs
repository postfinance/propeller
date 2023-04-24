use postgres::{Client, NoTls};

use crate::internal::database::{DatabaseClient, DatabaseConfig};

pub(crate) struct PostgresClient {
    client: Client,
}

impl DatabaseClient for PostgresClient {
    fn new(database_config: &DatabaseConfig) -> Self {
        PostgresClient {
            client: Client::connect(&database_config.url, NoTls)
                .expect("🛑 Failed to load PostgreSQL configuration!"),
        }
    }

    fn get_existing_users(&mut self, prefix: &str) -> Vec<String> {
        let result = self.client.query(
            "SELECT usename as role_name FROM pg_catalog.pg_user WHERE usename LIKE '$1%'",
            &[&prefix],
        );

        let mut existing_users = Vec::new();

        match result {
            Ok(rows) => {
                for row in &rows {
                    let username: String = row.get("username");
                    existing_users.push(username);
                }
            }
            Err(err) => {
                println!("🛑 Failed to retrieve existing users: {}", err);
            }
        }

        existing_users
    }

    fn create_user_and_assign_role(&mut self, username: &str, password: &str, role: &str) {
        let client = &mut self.client;

        create_user(username, password, client);
        grant_role(username, role, client);
    }

    fn drop_users(&mut self, users: Vec<String>) {
        for user in users {
            drop_user(user.as_str(), &mut self.client)
        }
    }
}

fn create_user(username: &str, password: &str, client: &mut Client) {
    client
        .execute("CREATE USER $1 WITH PASSWORD '$2'", &[&username, &password])
        .expect(format!("🛑 Failed to create user '{}'!", username).as_str());
    println!("✅ User '{}' successfully created", username);
}

fn grant_role(username: &str, role: &str, client: &mut Client) {
    client
        .execute("GRANT $1 TO $2", &[&role, &username])
        .expect(format!("🛑 Failed to grant role '{}' to user '{}'!", role, username).as_str());
    println!("✅ Role '{}' successfully granted to '{}'", role, username);
}

fn drop_user(username: &str, client: &mut Client) {
    client
        .execute("DROP USER $1", &[&username])
        .expect(format!("🛑 Failed to drop user '{}'!", username).as_str());
    println!("✅ User '{}' successfully dropped", username);
}
