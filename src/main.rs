// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use clap::Parser;
use env_logger::{Env, DEFAULT_WRITE_STYLE_ENV};

use crate::argo_cd::ArgoCD;
use crate::cli::{CliArgs, Command};
use crate::config::{read_config, Config};
use crate::vault::Vault;
use crate::workflow::rotate_secrets_using_switch_method;

mod argo_cd;
mod cli;
mod config;
mod database;
mod password;
mod vault;
mod workflow;

fn main() {
    init_logger();

    let args: CliArgs = CliArgs::parse();

    match args.command {
        Command::InitVault(int_args) => {
            let config: Config = read_config(int_args.base.config_path.clone());
            let mut vault: Vault = Vault::connect(&config);
            vault.init_secret_path()
        }
        Command::Rotate(rotate_args) => {
            let config: Config = read_config(rotate_args.base.config_path.clone());
            let mut argo_cd: ArgoCD = ArgoCD::init(&config);
            let mut vault: Vault = Vault::connect(&config);
            rotate_secrets_using_switch_method(&rotate_args, &config, &mut argo_cd, &mut vault)
        }
    }
}

fn init_logger() {
    let env = Env::default()
        .filter_or("PROPELLER_LOG_LEVEL", "error")
        .write_style_or("PROPELLER_LOG_STYLE", DEFAULT_WRITE_STYLE_ENV);

    env_logger::init_from_env(env);
}

#[cfg(test)]
mod tests {
    use std::fs::{read_dir, read_to_string};
    use std::path::Path;

    #[test]
    fn test_license_headers() {
        let expected_header = r#"// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

"#;

        let directories = ["src", "tests"];

        for dir in &directories {
            verify_all_files_in_directory_start_with_license_header(dir, expected_header);
        }
    }

    fn verify_all_files_in_directory_start_with_license_header(dir: &str, expected_header: &str) {
        let paths = read_dir(dir).unwrap();

        for path in paths {
            let path = path.unwrap().path();
            if should_process_path(&path) {
                if path.is_dir() {
                    verify_all_files_in_directory_start_with_license_header(
                        path.to_str().unwrap(),
                        expected_header,
                    );
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    verify_file_starts_with_license_header(&path, expected_header);
                }
            }
        }
    }

    fn should_process_path(path: &Path) -> bool {
        let path_str = path.to_str().unwrap();
        if path_str.starts_with("tests/utilities") || path_str.starts_with("tests\\utilities") {
            path_str == "tests/utilities/src"
                || path_str.starts_with("tests/utilities/src/")
                || path_str == "tests\\utilities\\src"
                || path_str.starts_with("tests\\utilities\\src/")
        } else {
            true
        }
    }

    fn verify_file_starts_with_license_header(path: &Path, expected_header: &str) {
        let contents = read_to_string(path).unwrap();
        assert!(
            contents.starts_with(expected_header),
            "File {} does not start with the expected license header",
            path.display()
        );
    }
}
