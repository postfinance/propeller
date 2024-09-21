// Copyright (c) 2024 PostFinance AG
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::{Command, Stdio};

#[test]
fn propeller_cli() {
    Command::cargo_bin("propeller")
        .unwrap()
        .stdout(Stdio::piped())
        .assert()
        .failure()
        .stderr(contains("propeller - Automated database secret rotation"))
        .stderr(contains("init-vault"))
        .stderr(contains(
            "Initialize a Vault path with the necessary structure for secret management",
        ))
        .stderr(contains("rotate"))
        .stderr(contains("Rotate PostgreSQL database secrets"))
        .stderr(contains("help"))
        .stderr(contains(
            "Print this message or the help of the given subcommand(s)",
        ))
        .stderr(contains("-h, --help"))
        .stderr(contains("Print help"))
        .stderr(contains("-V, --version"))
        .stderr(contains("Print version"));
}

#[test]
fn propeller_cli_init_vault_help() {
    Command::cargo_bin("propeller")
        .unwrap()
        .arg("init-vault")
        .arg("--help")
        .stdout(Stdio::piped())
        .assert()
        .success()
        .stdout(contains(
            "Initialize a Vault path with the necessary structure for secret management.",
        ))
        .stdout(contains(
            "This command prepares the Vault backend for subsequent secret rotation operations.",
        ))
        .stdout(contains("init-vault [OPTIONS]"))
        .stdout(contains("-c, --config-path <CONFIG_PATH>"))
        .stdout(contains("Path to the configuration file"))
        .stdout(contains("[default: config.yml]"))
        .stdout(contains("-h, --help"))
        .stdout(contains("Print help"))
        .stdout(contains("-V, --version"))
        .stdout(contains("Print version"));
}

#[test]
fn propeller_cli_rotate_help() {
    Command::cargo_bin("propeller")
        .unwrap()
        .arg("rotate")
        .arg("--help")
        .stdout(Stdio::piped())
        .assert()
        .success()
        .stdout(contains("Rotate PostgreSQL database secrets."))
        .stdout(contains("This command orchestrates the process of generating new secrets, updating the database, and storing the new secrets in Vault."))
        .stdout(contains("rotate [OPTIONS"))
        .stdout(contains(
            "-c, --config-path <CONFIG_PATH>",
        ))
        .stdout(contains("Path to the configuration file"))
        .stdout(contains("[default: config.yml]"))
        .stdout(contains("-p, --password-length <PASSWORD_LENGTH>"))
        .stdout(contains(
            "The length of the randomly generated alphanumeric password",
        ))
         .stdout(contains(
            "[default: 20]",
        ))
        .stdout(contains("-h, --help"))
        .stdout(contains("Print help"))
        .stdout(contains("-V, --version"))
        .stdout(contains("Print version"));
}
