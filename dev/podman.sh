#!/bin/bash

# Define volume names
POSTGRESQL_VOLUME="postgresql"
DEMO_DATA_VOLUME="demo-data"
VAULT_DATA_VOLUME="vault-data"

# Start Postgres for Vault
podman start postgres-vault || \
  podman run -d --name postgres-vault \
    -v "${POSTGRESQL_VOLUME}:/var/lib/postgresql/data" \
    -e POSTGRES_DB=vault \
    -e POSTGRES_USER=vault \
    -e POSTGRES_PASSWORD=vault_password \
    --restart on-failure:3 \
    postgres:12.19-alpine3.20

# Start Postgres for application (demo)
podman start postgres-demo || \
  podman run -d --name postgres-demo \
    -v "${DEMO_DATA_VOLUME}:/var/lib/postgresql/data" \
    -v "$(dirname "$0")/postgres:/docker-entrypoint-initdb.d" \
    -e POSTGRES_DB=demo \
    -e POSTGRES_USER=demo \
    -e POSTGRES_PASSWORD=demo_password \
    -p 5432:5432 \
    --restart on-failure:3 \
    postgres:12.19-alpine3.20

# Start Vault (waits for postgres-vault to be ready)
podman start vault || \
  podman run -d --name vault \
    --hostname vault \
    -v "${VAULT_DATA_VOLUME}:/vault/data" \
    -e VAULT_ADDR=http://localhost:8200 \
    -e DATABASE_DRIVER=postgres \
    -e DATABASE_URL=postgres://vault:vault_password@postgres-vault:5432/vault \
    -e VAULT_DEV_ROOT_TOKEN_ID=root-token \
    -p 8200:8200 \
    --restart on-failure:3 \
    hashicorp/vault:1.17.1
