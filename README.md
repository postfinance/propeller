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

## Target Architecture

`propeller` is designed to seamlessly integrate with a specific application deployment architecture commonly used in modern environments:

- **Containerized Applications:** Applications are deployed within a [Kubernetes](https://kubernetes.io/) cluster for scalability and efficient resource management.
- **PostgreSQL Database:** Data is persisted in a robust [PostgreSQL](https://www.postgresql.org/) database, known for its reliability and feature set.
- **Vault for Secrets Management:** Sensitive information like passwords and API keys are securely stored and managed within [Vault](https://www.hashicorp.com/products/vault) for enhanced security.
- **ArgoCD for GitOps Automation:** [ArgoCD](https://argoproj.github.io/cd/) is utilized for GitOps principles, enabling declarative management of infrastructure and applications through Git repositories. Importantly, ArgoCD can also manage the synchronization of secrets from Vault using plugins like [@postfinance/kubectl-vault_sync](https://github.com/postfinance/kubectl-vault_sync).

### Visual Representation

![Architecture](https://www.plantuml.com/plantuml/png/VP2nRi8m48PtFyKrB310QDKkKMG1OQ6b4XLLzqjoX0Z7jiuELLNrtHj722gGEjh_tVzzzinvPDysIjpvFJK4npfdr5u8TwYrHSO62jDOeqbx-1O000ii3XMRLfUPKORJrBAnf1Inb32OJXyNJtFn8uJjvh0YYB9Ld2rXez3l33VHgUPI6wL5A4e6d_lQaypMgpJkRsG4w23KtqEQmfa3Klu5lBGviIPFxgPwRgsg2_IrqQ4AhRqUuCfaoskX3soLX-sNBc3uRF9Hxt5qtLdyxxvgEg4R-uSR-z1oOiDa8eDO0g-eksrtdVNSSmh3EDum1RSuXhqnXr7uYn8zvkW8DiRvgilVre5UvkBYshzAY8u5ux7iiWIstZ11M1Oz1jAGz8C9l3DgjoC6HmMJTs96kcmRzGi0)

<small><a href="./docs/application-architecture.puml">Source</a>.</small>

`propeller` can either run as a CronJob in Kubernetes, or as part of any scheduled pipeline within your CI/CD environment.

## Configuration

Propeller relies on a [configuration file](#configuration-file-yaml) and [environment variables](#vault-authentication-token-vault_token) to function correctly.

Once you're done configuring, [initialize Vault](#initializing-vault-for-secret-management).
And that's it: Proceed to [rotate your secrets](#rotating-secrets).

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
propeller init-vault [OPTIONS]
```

#### Options

#### Result

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

The "TBD" placeholders indicate that these values _must_ be filled once with the initial values before continuing the [rotation process](#rotating-secrets).

**Screenshot of initialized Vault secret:**

[![Initial Vault Structure](img/initial-vault-structure.png)](img/initial-vault-structure.png)

### Rotating Secrets

Once Vault has been initialized, you're ready to frequently rotate your database secrets.

**Command Usage:**

```cookie
propeller rotate [OPTIONS]
```

#### Options

#### Sequence Diagram

!["switch" Workflow](https://www.plantuml.com/plantuml/png/nLF1RjGm4BtxAmPnWKFLRWMNFQ2M8d7X02nmT-qXiUHuZ3tUYh_7cpJOHbQ4gfLwoMx6y-Qzl7c-YIm3fycA5ppYX70qzq4w5iBdkb76vnVu7CYZjHZWvTNLc_TlRvlJ7p9PRlifyX3myELJKxuD0zrzQ4lUMwCa6t92E684EcAeo_lw1LB4U7e4s0aX5PkZP2pwX2XIBzujolRm5NybZ0npFyxmWbtKpq-uo9Y_0_OhZyQskIMf0H_HOJZrPGirJU1bZ0yKT8kexCdQY3DWeRekW9MnjhBy_LVe8Ic5CHQbDQxloNUlDtbhMxRPIdUNwF1WM8srzy3qo7jEkYLSU_WMp33aKY1hAN6XU4pVyfCHRSWElvqglTNH2jXKLHE82lp44BRI5gywtzyGjR6w8-TGCMJpnsBsTgPQtdN6F1qZotjhueYw7xAw-lGxVIs49V9vhhMG71kxhX4KJTxYQLO0DXFcMc__nUSLc9LpYjqOTBOwDyChZquRDrp6PSkNFwMng5ztjetkVs_txfbkz-vSjxjKIh-uGQVJPFy0)

<small><a href="./docs/switch-workflow.puml">Source</a>.</small>
