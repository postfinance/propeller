use clap::{Parser, Subcommand};

/// propeller - Automated database secret rotation.
///
/// This tool simplifies the process of managing and rotating secrets for PostgreSQL databases, leveraging Vault as a secure backend.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help(true))] // Require at least one subcommand
#[command(propagate_version = true)] // Display version in subcommand help
pub(crate) struct CliArgs {
    #[clap(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Initialize a Vault path with the necessary structure for secret management.
    ///
    /// This command prepares the Vault backend for subsequent secret rotation operations.
    InitVault(InitVaultArgs),

    /// Rotate PostgreSQL database secrets.
    ///
    /// This command orchestrates the process of generating new secrets, updating the database, and storing the new secrets in Vault.
    Rotate(RotateArgs),
}

/// Base arguments for subcommands that share common parameters.
#[derive(Parser, Debug)]
pub(crate) struct BaseArgs {
    /// Path to the configuration file (default: config.yml).
    #[clap(short, long, default_value = "config.yml")]
    pub(crate) config_path: std::path::PathBuf,
}

/// Arguments specific to the `rotate` subcommand.
#[derive(Parser, Debug)]
pub(crate) struct RotateArgs {
    #[clap(flatten)] // Inherit arguments from BaseArgs
    pub(crate) base: BaseArgs,

    /// The length of the randomly generated alphanumeric password
    #[clap(short, long, default_value = "20")]
    pub(crate) password_length: usize,
}

/// Arguments specific to the `init-vault` subcommand.
#[derive(Parser, Debug)]
pub(crate) struct InitVaultArgs {
    #[clap(flatten)] // Inherit arguments from BaseArgs
    pub(crate) base: BaseArgs,
}
