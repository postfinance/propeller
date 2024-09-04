# Developer Instructions

This guide will help you set up your development environment for the project.

**Contents:**

- [Prerequisites](#prerequisites)
- [Preparing the Repository](#preparing-the-repository)
- [Environment Setup](#environment-setup)
  - [With `podman`](#setting-up-with-podman)
  - [With Docker](#setting-up-with-docker)
- [Building the Project](#building-the-project)
  - [Building a Binary](#building-a-binary)
  - [Running Tests](#running-tests)
  - [Running the CLI](#running-the-cli)

## Prerequisites

Before you start, ensure you have the following installed:

- [Git](https://git-scm.com/downloads): For version control
  - [Git LFS](https://git-lfs.com/): For managing large files within the project
- [Node.js](https://nodejs.org/en/download): For running certain project scripts (including dependency management)
- [`podman`](https://podman.io/docs/installation) or [Docker](https://www.docker.com/products/docker-desktop/): For containerization of the Vault instance and databases
  - This is especially required for [integration testing](#running-tests)

## Preparing the Repository

1. Make sure you have Git Large File Storage (LFS) installed beforehand:

```shell
git lfs install
git lfs fetch
```

This ensures that when cloning the repository later, large files needed by the project are cloned too.

2. Clone the `propeller` project from GitHub:

```shell
git clone git@github.com:postfinance/propeller.git
```

3. Navigate into the project directory:

```
cd propeller
```

4. Initialize Git Submodules:

```shell
git submodule init
git submodule update
```

The project is connected to [argoproj/argo-cd](https://github.com/argoproj/argo-cd) and thus includes some of its sources.

5. Install project dependencies:

```shell
npm ci --cache .npm
```

**Note:** The `--cache .npm` option helps speed up subsequent installations.

## Environment Setup

`propeller` requires the following components for development:

- **Two PostgreSQL databases:**
  - One for Vault's backend storage
  - One to simulate the database of an application, used for secret rotation
- **A Vault instance:** For managing secrets
- **An ArgoCD Instance:** That manages the productive application

Two options are provided for setting up the environment, either using `podman` or `docker-compose`.
Refer to the respective scripts ([`dev/podman.sh`](dev/podman.sh) and [`dev/docker-compose.yml`](dev/docker-compose.yml)) for detailed instructions.

**Notes:**

- If using any of these options, Vault will be accessible on http://localhost:8200.
- The provided "root-token" is for development only.
  Use strong, unique tokens in production and follow best practices for Vault token management.
- The demo database is initialized with sample users and credentials for demonstration purposes.
  After [having initialized Vault](#running-the-cli), you could configure these users for rotation, e.g. with the following secret value in `path/to/my/secret`:

```json
{
  "postgresql_active_user": "user1",
  "postgresql_active_user_password": "initialpw",
  "postgresql_user_1": "user1",
  "postgresql_user_1_password": "initialpw",
  "postgresql_user_2": "user2",
  "postgresql_user_2_password": "initialpw"
}
```

- The dev deployment makes use of [Counterfact](https://counterfact.dev) instead of providing a full-fletched Kubernetes with ArgoCD installed.
  If you have a development instance of Kubernetes available, take a look at the ["ArgoCD Getting Started"](./argo-cd/docs/getting_started.md) section for more information.

### Setting up with `podman`:

```shell
./dev/podman.sh
```

### Setting up with Docker:

```shell
docker compose up -f dev/docker-compose.yml
```

## Building the Project

With your development environment up and running, you can now proceed with building and running the CLI.

### Building a Binary

Use Cargo, Rust's package manager and build tool, to compile the project:

```shell
cargo build
```

This will create the executable binary in the `target/debug` directory.
To run the compiled binary, execute:

```shell
./target/debug/propeller
```

### Running Tests

Cargo makes it easy to run the project's unit and integration tests:

```shell
cargo test
```

Note that the integration tests make use of [Testcontainers](https://testcontainers.com).
K3s, Vault and PostgreSQL will be deployed automatically using it.

Once you access to a virtualization software such as Docker, you can execute the integration tests without further ado.

#### Debugging ArgoCD

For development and debugging purposes it's also good to be able to take a look at ArgoCD sometimes.
You can install ArgoCD into Kubernetes using the below command:

```shell
kubectl apply -f tests/resources/argocd.deployment.yml
```

Next, extract the initial password for ArgoCD (see ["ArgoCD Getting Started"](./argo-cd/docs/getting_started.md) for more information):

```shell
kubectl get secret argocd-initial-admin-secret -o jsonpath={.data.password} | base64 -d
```

Create a `port-forward` and access the ArgoCD UI in a browser:

```shell
kubectl port-forward svc/argocd-server :80
```

#### A Note for Windows Users

If testcontainers fail to connect to your Docker socket on Windows, add the below environment variable to the test command:

```shell
DOCKER_HOST=tcp://localhost:2375 cargo test
```

#### And a Note for Linux Users

You'll need to "fake" a Docker socket using `podman` (_if_ you're using `podman`, of course).
Invoke `podman system service --time=0` directly to create a live-socket without using `systemd`.
If you do not pass another parameter, the default location will be used to create the socket file.
You can use the below commands _in another terminal_ to connect to the socket (according to [the docs](https://docs.podman.io/en/latest/markdown/podman-system-service.1.html#run-the-command-directly)).

```shell
export DOCKER_HOST=unix://$XDG_RUNTIME_DIR/podman/podman.sock
export TESTCONTAINERS_RYUK_DISABLED=true
cargo test
```

### Running the CLI

To run the CLI without compiling the binary each time, execute:

```shell
VAULT_TOKEN=root-token cargo run -- init-vault -c dev/config.yml
```

This will also pick up a development configuration perfectly fitting for the previously installed [development environment](#environment-setup).
