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
