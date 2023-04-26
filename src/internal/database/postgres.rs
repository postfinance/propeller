use postgres::{Client, NoTls};

use crate::internal::database::{DatabaseClient, DatabaseConfig};
use crate::CLI_ARGS;

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
        let mut existing_users = Vec::new();

        if CLI_ARGS.dry_run {
            println!("🧪 Adding a test user for dry run, not actually querying anything");
            existing_users.push("dry-run-user".to_string());
            return existing_users;
        } else {
            println!("✅ Reading existing users from database")
        }

        let result = self.client.query(
            "SELECT usename as role_name FROM pg_catalog.pg_user WHERE usename LIKE '$1%'",
            &[&prefix],
        );

        match result {
            Ok(rows) => {
                for row in &rows {
                    let username: String = row.get("username");

                    if CLI_ARGS.verbose {
                        println!("👀 Found existing username: {}", username)
                    }

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
        if CLI_ARGS.dry_run {
            println!("🧪 Would assign role '{}' to user '{}", role, username);
            return;
        }

        let client = &mut self.client;

        create_user(username, password, client);
        grant_role(username, role, client);
    }

    fn drop_users(&mut self, users: Vec<String>) {
        if CLI_ARGS.dry_run {
            println!("🧪 Would drop {} now unused users", users.len());
            return;
        }

        for user in users {
            drop_user(user.as_str(), &mut self.client)
        }
    }
}

fn create_user(username: &str, password: &str, client: &mut Client) {
    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Create user '{}' with random password", username);
    }

    client
        .execute("CREATE USER $1 WITH PASSWORD '$2'", &[&username, &password])
        .expect(format!("🛑 Failed to create user '{}'!", username).as_str());

    if CLI_ARGS.verbose {
        println!("👀 User '{}' successfully created", username);
    }
}

fn grant_role(username: &str, role: &str, client: &mut Client) {
    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Grant role '{}' to user '{}'", role, username);
    }

    client
        .execute("GRANT $1 TO $2", &[&role, &username])
        .expect(format!("🛑 Failed to grant role '{}' to user '{}'!", role, username).as_str());

    if CLI_ARGS.verbose {
        println!("👀 Role '{}' successfully granted to '{}'", role, username);
    }
}

fn drop_user(username: &str, client: &mut Client) {
    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Drop user '{}'", username);
    }

    client
        .execute("DROP USER $1", &[&username])
        .expect(format!("🛑 Failed to drop user '{}'!", username).as_str());

    if CLI_ARGS.verbose {
        println!("👀 User '{}' successfully dropped", username);
    }
}
