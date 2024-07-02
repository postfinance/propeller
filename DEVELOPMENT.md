# Developer Instructions

This guide will help you set up your development environment for the project.

## Prerequisites

Before you start, ensure you have the following installed:

- [Git](https://git-scm.com/downloads): For version control
  - Git LFS: For managing large files within the project
- [Node.js](https://nodejs.org/en/download: For running certain project scripts (including dependency management)
- [`podman`](https://podman.io/docs/installation) or [Docker](https://www.docker.com/products/docker-desktop/): For containerization of the Vault instance and databases

## Cloning the Repository

1. Clone the `propeller` project from GitHub:

```shell
git clone git@github.com:postfinance/propeller.git
```

2. Navigate into the project directory:

```
cd propeller
```

3. Initialize Git Large File Storage (LFS):

```shell
git lfs install
git lfs fetch
```

This ensures you have the large files needed by the project.

4. Install project dependencies (required if you're working with project resources):

```shell
npm ci --cache .npm
```

**Note:** The --cache .npm option helps speed up subsequent installations.

## Environment Setup

`propeller` requires the following components for development:

- **Two PostgreSQL databases:**
  - One for Vault's backend storage
  - One to simulate the database of an application, used for secret rotation
- **A Vault instance:** For managing secrets

Note that if using any of the below options, Vault will be accessible on http://localhost:8200.
Extract the root token from the logs of the container.

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
cargo tests
```

Cargo will automatically discover and execute the tests defined within the project.

### Running the CLI

To run the CLI without compiling the binary each time, execute:

```shell
cargo run -- init-vault -c dev/config.yml
```

This will also pick up a development configuration perfectly fitting for the previously installed [development environment](#environment-setup).
