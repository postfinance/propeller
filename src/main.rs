use clap::Parser;
use env_logger::{Env, DEFAULT_WRITE_STYLE_ENV};

use config::Config;

use crate::cli::{CliArgs, Command};
use crate::config::read_config;
use crate::vault::Vault;
use crate::workflow::switch_active_user;

mod cli;
mod config;
mod password;
mod vault;
mod workflow;

fn main() {
    init_logger();

    let args: CliArgs = CliArgs::parse();

    match args.command {
        Command::InitVault(int_args) => {
            let config: Config = read_config(int_args.base.config_path);
            let mut vault: Vault = Vault::connect(&config);
            vault.init_secret_path()
        }
        Command::Rotate(rotate_args) => {
            let config: Config = read_config(rotate_args.base.config_path);
            let vault: Vault = Vault::connect(&config);
            switch_active_user(&config, &vault)
        }
    }
}

fn init_logger() {
    let env = Env::default()
        .filter_or("PROPELLER_LOG_LEVEL", "error")
        .write_style_or("PROPELLER_LOG_STYLE", DEFAULT_WRITE_STYLE_ENV);

    env_logger::init_from_env(env);
}
