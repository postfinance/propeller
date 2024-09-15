#!/usr/bin/env sh

set -ex

release_version=$1
image_name="ghcr.io/postfinance/propeller:$release_version"

echo "$GITHUB_TOKEN " | docker login ghcr.io -u "$GITHUB_USERNAME" --password-stdin

docker build --build-arg "RELEASE_VERSION=$release_version" -t "$image_name" .
docker push "$image_name"
