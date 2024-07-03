use serde::Deserialize;
use std::{fs::File, io::Read, path::PathBuf};
use log::debug;

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
    pub(crate) jdbc_url: String,
}

pub(crate) fn read_config(config_path: PathBuf) -> Config {
    let path_string = config_path.clone().into_os_string().into_string().unwrap();
    debug!("Reading config at: {path_string}");

    let mut config_data: String = String::new();
    let mut config_file: File = File::open(config_path).expect("Failed to read configuration file");
    config_file
        .read_to_string(&mut config_data)
        .expect("Failed to read configuration file");

    serde_yaml::from_str(&config_data).expect("Failed to parse configuration")
}
