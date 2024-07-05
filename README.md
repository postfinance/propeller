<h1 align="center">
  propeller - automated database secret rotation.
</h1>

<p align="center">
  <img src="img/logo-circle.png" alt="propeller-logo" width="120px" height="120px" style="border-radius: 50%;" />
  <br />
  <i>
    propeller is a secret rotation tool for applications running in Kubernetes,
    <br/>using HashiCorp Vault and ArgoCD.
  </i>
</p>

<p align="center">
  <a href="https://github.com/postfinance/propeller/actions/workflows/build.yml">
    <img src="https://github.com/postfinance/propeller/actions/workflows/build.yml/badge.svg" alt="Rust Build" />
  </a>
  <a href="https://github.com/postfinance/propeller/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/postfinance/propeller" alt="MIT License">
  </a>
  <a href="https://github.com/postfinance/propeller/releases">
    <img src="https://img.shields.io/github/v/release/postfinance/propeller" alt="GitHub Release">
  </a>
</p>

<hr>

## Configuration

Propeller relies on a configuration file and an environment variable to function correctly.

### Configuration File (YAML)

All [commands](#commands) in `propeller` accept the `-c <config_path>` argument to specify the location of your configuration file.
If you don't provide the argument, the tool will default to `config.yml` in the current directory.

The configuration file is in YAML format and has the following structure:

```yaml
postgres:
  host: 'localhost' # Replace with your database host
  port: 5432 # Replace with your database port
  database: 'demo' # Replace with your database
vault:
  address: 'http://localhost:8200' # Replace with your Vault address
  path: 'path/to/my/secret' # Replace with the desired path in Vault
```

Make sure to replace the placeholder values with your actual database connection details and the desired Vault path.

### Vault Authentication Token (`VAULT_TOKEN`)

In addition to the configuration file, Propeller requires a `VAULT_TOKEN` environment variable.
This token is used to authenticate with your Vault instance.

**Setting the `VAULT_TOKEN`:**

Before running any Propeller command, you need to set the `VAULT_TOKEN` environment variable.
Here's how you can do it in your shell:

```shell
export VAULT_TOKEN=<your_vault_token>
```

Replace `<your_vault_token>` with your actual Vault token.
**And make sure to keep this token secure!**
_Never_ include your `VAULT_TOKEN` directly in the configuration file or commit it to version control.

## Commands

### Initializing Vault for Secret Management

The `propeller init-vault` command is the first step in setting up your database secret rotation process.
It creates the necessary structure within your Vault instance to securely store and manage your PostgreSQL credentials.

**Command Usage:**

```cookie
propeller init-vault
```

After running the command, the specified Vault path will contain a JSON secret with the following structure:

```json
{
  "postgresql_active_user": "TBD",
  "postgresql_active_user_password": "TBD",
  "postgresql_user_1": "TBD",
  "postgresql_user_1_password": "TBD",
  "postgresql_user_2": "TBD",
  "postgresql_user_2_password": "TBD"
}
```

**Note that any previously present secrets in this path will be lost in the process!**

The "TBD" placeholders indicate that these values _must_ be filled once with the initial values before continuing the rotation process.

#### Example Result

[![Initial Vault Structure](img/initial-vault-structure.png)](img/initial-vault-structure.png)
