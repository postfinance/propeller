FROM debian:12.7-slim

ARG RELEASE_VERSION

LABEL org.opencontainers.image.title="propeller"
LABEL org.opencontainers.image.source="https://github.com/postfinance/propeller"
LABEL org.opencontainers.image.description="A secret rotation tool for applications running in Kubernetes, using HashiCorp Vault and ArgoCD."
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.version="$RELEASE_VERSION"
LABEL org.opencontainers.image.vendor="PostFinance AG"

RUN apt-get update \
    && apt-get upgrade -y \
    && rm -rf /var/lib/apt/lists/*

COPY target/release/propeller /usr/local/bin/propeller

ENTRYPOINT ["propeller"]
