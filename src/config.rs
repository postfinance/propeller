use log::debug;
use serde::Deserialize;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub(crate) postgres: PostgresConfig,
    pub(crate) vault: VaultConfig,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct VaultConfig {
    pub(crate) address: String,
    pub(crate) path: String,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct PostgresConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) database: String,
}

pub(crate) fn read_config(config_path: PathBuf) -> Config {
    let path_string = config_path.clone().into_os_string().into_string().unwrap();
    debug!("Reading config at: {path_string}");

    let mut config_data: String = String::new();
    let mut config_file: File = File::open(config_path)
        .expect(format!("Failed to read configuration file: '{path_string}'").as_str());
    config_file
        .read_to_string(&mut config_data)
        .expect("Failed to read configuration file");

    serde_yaml::from_str(&config_data).expect("Failed to parse configuration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(
        expected = "Failed to read configuration file: 'tests/resources/config/non_existing.yml'"
    )]
    fn read_config_invalid_file() {
        read_config(PathBuf::from("tests/resources/config/non_existing.yml"));
    }

    #[test]
    #[should_panic(expected = "Failed to parse configuration: Error(\"missing field `postgres`")]
    fn read_config_missing_postgresql() {
        read_config(PathBuf::from(
            "tests/resources/config/missing_postgresql.yml",
        ));
    }

    #[test]
    #[should_panic(expected = "Failed to parse configuration: Error(\"missing field `vault`")]
    fn read_config_missing_vault() {
        read_config(PathBuf::from("tests/resources/config/missing_vault.yml"));
    }
}
