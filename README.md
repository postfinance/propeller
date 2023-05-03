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

The idea is actually very simple. Imagine having a running application in [Kubernetes](https://kubernetes.io/). It must
access a database to store some information in it. We're also assuming three more things:

* All access information is stored in [HashiCorp Vault](https://www.hashicorp.com/products/vault) secrets
* You're using [ArgoCD](https://argo-cd.readthedocs.io/en/stable/) for namespace synchronization
* Your ArgoCD synchronization includes secrets from HashiCorp Vault
    * e.g. using [the `vault_sync` plugin](https://github.com/postfinance/kubectl-vault_sync)

### Secret Rotation

Now, how would you rotate the database secrets (e.g. username/password) without any downtime? It is
pretty simple, actually. Just use two different users concurrently - for a short while. The tool
supports two different workflows, based on your security requirements. **But,** it is only
supporting [PostgreSQL](https://www.postgresql.org/) for the time being.

* The ["exchange workflow"](#exchange) uses two predefined users (requires **no** `CREATE/DROP USER` grants).
* The ["rotate workflow"](#rotate) does replace users and passwords (requires `CREATE/DROP USER` grants).

Have a look at the workflow descriptions. That is it, already. Zero downtime secret rotation ✔️

#### Exchange

1. Detect the currently active user based on the values in HashiCorp Vault
2. Update the password of the passive user
3. Push the access information to a secret in HashiCorp Vault
4. Trigger an application rollout using ArgoCD, updating the active user
5. Change the password of the (now) passive user
6. Push the access information to another secret in HashiCorp Vault

##### Prerequisites

The database grants, roles and users listed in the following table are required.

| Name              | Usage                                                                    | Privileges                               |
|-------------------|--------------------------------------------------------------------------|------------------------------------------|
| `database_owner`  | Owner of the database, default user.                                     | all privileges                           | 
| `application_dml` | `NO LOGIN` role, basic role for `GRANT X TO application_dml` statements. | DML privileges                           | 
| `application_a`   | An "application runtime user" with an initial password.                  | `GRANT application_dml to application_a` | 
| `application_b`   | Another "application runtime user" with an initial password.             | `GRANT application_dml to application_b` | 

It is also recommended to use the following HashiCorp Vault secret structure.

```json
{
  "database.active.username": "application_a",
  "database.active.password": "MY_PASSWORD",
  "database.passive.username": "application_b",
  "database.passive.password": "ANOTHER_PASSWORD",
  "database.bench.password": ""
}
```

##### Configuration

`// TODO`

##### Execution

`// TODO`

#### Rotate

1. Create a new database username/password combination, using a prefix plus random part for the username
2. Assign the desired roles to the new user
3. Push the access information to a secret in HashiCorp Vault
4. Trigger an application rollout using ArgoCD
5. Remove the old username/password combination

##### Prerequisites

The database grants, roles and users listed in the following table are required.

| Name              | Usage                                                        | Privileges                               |
|-------------------|--------------------------------------------------------------|------------------------------------------|
| `database_owner`  | Owner of the database, default user.                         | all privileges                           | 
| `application_dml` | `NO LOGIN` role, basic role for `GRANT X TO Y` statements.   | DML privileges                           | 

**Note:** In case of errors, it may happen that old users will not correctly be cleaned up from the
database. It will require manual cleanup if that happens.

##### Configuration

`// TODO`

##### Execution

`// TODO`
