#!/usr/bin/env sh

set -ex

release_version=$1

npm run openapi:generate:argocd
cargo bump "${release_version}"
cargo build --release
