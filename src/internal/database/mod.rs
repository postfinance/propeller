mod postgres;

struct DatabaseConfig {
    url: String,
}

impl DatabaseConfig {
    pub fn new(url: String) -> Self {
        DatabaseConfig {
            url: url,
        }
    }
}

trait DatabaseClient {
    fn new(database_config: &DatabaseConfig) -> Self;
    fn get_existing_users(prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn create_user(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn delete_users_from_postgres(users: Vec<String>) -> Result<(), Box<dyn std::error::Error>>;
}
