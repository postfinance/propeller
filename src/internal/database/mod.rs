pub(crate) mod postgres;

pub(crate) struct DatabaseConfig {
    url: String,
}

impl DatabaseConfig {
    pub fn new(url: &str) -> Self {
        DatabaseConfig {
            url: url.to_string(),
        }
    }
}

pub(crate) trait DatabaseClient {
    fn new(database_config: &DatabaseConfig) -> Self;
    fn get_existing_users(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn create_user_and_assign_role(
        &mut self,
        username: &str,
        password: &str,
        role: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn drop_users(&mut self, users: Vec<String>) -> Result<(), Box<dyn std::error::Error>>;
}
