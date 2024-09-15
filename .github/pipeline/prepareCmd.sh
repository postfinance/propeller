#!/usr/bin/env sh

set -ex

release_version=$1

npm run openapi:generate:argocd
cargo bump "${release_version}"
cargo build --release

# Calculate MD5 Hash
md5sum "target/release/propeller" | awk '{print $1}' > "target/release/propeller.md5"
