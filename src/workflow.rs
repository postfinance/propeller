use log::{debug, info, trace};

use crate::argo_cd::ArgoCD;
use crate::cli::RotateArgs;
use crate::config::Config;
use crate::database::PostgresClient;
use crate::password::generate_random_password;
use crate::vault::{Vault, VaultStructure};

pub(crate) fn rotate_secrets_using_switch_method(
    rotate_args: &RotateArgs,
    config: &Config,
    argo_cd: &mut ArgoCD,
    vault: &mut Vault,
) {
    let db: PostgresClient = PostgresClient::init(config);

    info!("Starting 'switch' workflow");

    let mut secret: VaultStructure = vault.read_secret().unwrap_or_else(|_| {
        panic!(
            "Failed to read path '{}' - did you init Vault?",
            config.vault.clone().path
        )
    });

    if secret.postgresql_active_user != secret.postgresql_user_1
        && secret.postgresql_active_user != secret.postgresql_user_2
    {
        panic!("Failed to detect active user - did neither match user 1 nor 2")
    }

    let new_password: String = generate_random_password(rotate_args.password_length);

    update_passive_user_postgres_password(&db, &mut secret, new_password);
    switch_active_user(&mut secret);

    vault
        .write_secret(&secret)
        .expect("Failed to kick-off rotation workflow by switching active user - Vault is in an invalid state");

    debug!("Active and passive users switched and synchronized into Vault");
    debug!("Starting ArgoCD rollout now");

    argo_cd.sync();
    argo_cd.wait_for_rollout();

    debug!("ArgoCD rollout succeeded, continue changing password of previously active user");

    let new_password: String = generate_random_password(rotate_args.password_length);

    update_passive_user_postgres_password(&db, &mut secret, new_password);

    vault
        .write_secret(&secret)
        .expect("Failed to update PASSIVE user password after sync - Vault is in an invalid state");

    println!("Successfully rotated all secrets")
}

fn switch_active_user(secret: &mut VaultStructure) {
    if secret.postgresql_active_user == secret.postgresql_user_1 {
        secret
            .postgresql_active_user
            .clone_from(&secret.postgresql_user_2);
        secret
            .postgresql_active_user_password
            .clone_from(&secret.postgresql_user_2_password);
    } else {
        secret
            .postgresql_active_user
            .clone_from(&secret.postgresql_user_1);
        secret
            .postgresql_active_user_password
            .clone_from(&secret.postgresql_user_1_password);
    }

    trace!("Switched active and passive user in Vault secret (locally)")
}

fn update_passive_user_postgres_password(
    db: &PostgresClient,
    secret: &mut VaultStructure,
    new_password: String,
) {
    info!("Rotating database password of passive user");

    let (passive_user, passive_user_password) =
        if secret.postgresql_active_user == secret.postgresql_user_1 {
            let original_password = secret.postgresql_user_2_password.clone();
            secret.postgresql_user_2_password.clone_from(&new_password);
            (secret.postgresql_user_2.clone(), original_password)
        } else {
            let original_password = secret.postgresql_user_1_password.clone();
            secret.postgresql_user_1_password.clone_from(&new_password);
            (secret.postgresql_user_1.clone(), original_password)
        };

    let mut conn = db.connect_for_user(passive_user.clone(), passive_user_password);
    let query = format!("ALTER ROLE {passive_user} WITH PASSWORD '{new_password}'");

    conn.execute(query.as_str(), &[])
        .unwrap_or_else(|_| panic!("Failed to update password of '{}'", passive_user));

    trace!("Successfully rotated database password of passive user");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_active_user_user1_active() {
        let mut secret: VaultStructure = create_vault_structure_active_user_1();

        switch_active_user(&mut secret);

        assert_eq!(secret.postgresql_active_user, "user2");
        assert_eq!(secret.postgresql_active_user_password, "password2");
    }

    #[test]
    fn switch_active_user_user2_active() {
        let mut secret: VaultStructure = create_vault_structure_active_user_2();

        switch_active_user(&mut secret);

        assert_eq!(secret.postgresql_active_user, "user1");
        assert_eq!(secret.postgresql_active_user_password, "password1");
    }

    // #[test]
    // fn update_passive_user_password_user1_active() {
    //     let client = PropellerDBClient{};
    //
    //     let mut secret: VaultStructure = create_vault_structure_active_user_1();
    //
    //     let new_password = "new_password".to_string();
    //
    //     update_passive_user_postgres_password(client, & mut secret, new_password.clone());
    //
    //     assert_eq!(secret.postgresql_active_user, "user1");
    //     assert_eq!(secret.postgresql_active_user_password, "password1");
    //     assert_eq!(secret.postgresql_user_2_password, new_password);
    // }
    //
    // #[test]
    // fn update_passive_user_password_user2_active() {
    //     let client = PropellerDBClient{};
    //
    //     let mut secret: VaultStructure = create_vault_structure_active_user_2();
    //
    //     let new_password = "new_password".to_string();
    //
    //     update_passive_user_postgres_password(client,&mut secret, new_password.clone());
    //
    //     assert_eq!(secret.postgresql_active_user, "user2");
    //     assert_eq!(secret.postgresql_active_user_password, "password2");
    //     assert_eq!(secret.postgresql_user_1_password, new_password);
    // }

    fn create_vault_structure_active_user_1() -> VaultStructure {
        let secret = VaultStructure {
            postgresql_active_user: "user1".to_string(),
            postgresql_active_user_password: "password1".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };
        secret
    }

    fn create_vault_structure_active_user_2() -> VaultStructure {
        let secret = VaultStructure {
            postgresql_active_user: "user2".to_string(),
            postgresql_active_user_password: "password2".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };
        secret
    }
}
