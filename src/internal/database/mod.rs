use serde::{Deserialize, Serialize};

pub(crate) mod postgres;

#[derive(Debug, Serialize, Deserialize, Default)]
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
    fn get_existing_users(&mut self, prefix: &str) -> Vec<String>;
    fn create_user_and_assign_role(&mut self, username: &str, password: &str, role: &str);
    fn update_user_password(&mut self, username: &str, password: &str);
    fn drop_users(&mut self, users: Vec<String>);
}
