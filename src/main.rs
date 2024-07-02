use clap::Parser;

use config::Config;

use crate::cli::{CliArgs, Command};
use crate::config::read_config;
use crate::vault::Vault;

mod cli;
mod config;
mod vault;

fn main() {
    let args: CliArgs = CliArgs::parse();

    match args.command {
        Command::InitVault(int_args) => {
            let config: Config = read_config(int_args.base.config_path);
            let mut vault: Vault = Vault::connect(&config);
            vault.init_secret_path()
        }
        Command::Rotate(_) => {}
    }
}
