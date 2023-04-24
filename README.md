<h1 align="center">
  propeller - automated database secret rotations.
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
</p>

<hr>

## How does it work?

### Setup

The idea is actually very simple. Imagine having a running application in [Kubernetes](https://kubernetes.io/). It must
access a database to store some information in it. We're also assuming three more things:

* All access information is stored in [HashiCorp Vault](https://www.hashicorp.com/products/vault) secrets
* You're using [ArgoCD](https://argo-cd.readthedocs.io/en/stable/) for namespace synchronisation
* Your ArgoCD synchronization includes secrets from HashiCorp Vault
    * e.g. using [the `vault_sync` plugin](https://github.com/postfinance/kubectl-vault_sync)

### Secret Rotation

Now, how would you rotate the database secrets (e.g. username/password) without any downtime?

1. Create a new database username/password combination, using a prefix plus random part for the username
2. Assign the desired roles to the new user
3. Push the access information to a secret in HashiCorp Vault
4. Trigger an application rollout using ArgoCD
5. Remove the old username/password combination

That's it. Zero downtime secret rotation ✔️

**Note:** It's only supporting [PostgreSQL](https://www.postgresql.org/) right now.
