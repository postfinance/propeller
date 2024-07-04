use crate::cli::RotateArgs;
use crate::config::Config;
use crate::password::generate_random_password;
use crate::vault::{Vault, VaultStructure};
use log::debug;
use vaultrs::auth::userpass::user::update_password;

pub(crate) fn rotate_secrets_using_switch_method(
    rotate_args: &RotateArgs,
    config: &Config,
    vault: &mut Vault,
) {
    debug!("Starting 'switch' workflow");

    let vault_path = config.vault.clone().path;
    let mut secret: VaultStructure = vault
        .read_secret()
        .expect(format!("Failed to read path '{vault_path}' - did you init Vault?").as_str());

    if secret.postgresql_active_user != secret.postgresql_user_1
        && secret.postgresql_active_user != secret.postgresql_user_2
    {
        panic!("Failed to detect active user - did neither match user 1 nor 2")
    }

    let new_password: String = generate_random_password(rotate_args.password_length);

    // TODO: PostgreSQL password change

    update_passive_user_password(&mut secret, new_password);
    switch_active_user(&mut secret);

    vault
        .write_secret(&secret)
        .expect("Failed to kick-off rotation workflow by switching active user");

    // TODO: Trigger ArgoCD Sync

    let new_password: String = generate_random_password(rotate_args.password_length);

    // TODO: PostgreSQL password change

    update_passive_user_password(&mut secret, new_password);
    vault
        .write_secret(&secret)
        .expect("Failed to update PASSIVE user password after sync");

    println!("Successfully rotated all secrets")
}

fn switch_active_user(secret: &mut VaultStructure) {
    if secret.postgresql_active_user == secret.postgresql_user_1 {
        secret.postgresql_active_user = secret.postgresql_user_2.clone();
        secret.postgresql_active_user_password = secret.postgresql_user_2_password.clone()
    } else {
        secret.postgresql_active_user = secret.postgresql_user_1.clone();
        secret.postgresql_active_user_password = secret.postgresql_user_1_password.clone()
    }
}

fn update_passive_user_password(secret: &mut VaultStructure, new_password: String) {
    if secret.postgresql_active_user == secret.postgresql_user_1 {
        secret.postgresql_user_2_password = new_password.clone();
    } else {
        secret.postgresql_user_1_password = new_password.clone();
    }
}

mod test {
    use super::*;

    #[test]
    fn test_switch_active_user_user1_active() {
        let mut secret = VaultStructure {
            postgresql_active_user: "user1".to_string(),
            postgresql_active_user_password: "password1".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };

        switch_active_user(&mut secret);

        assert_eq!(secret.postgresql_active_user, "user2");
        assert_eq!(secret.postgresql_active_user_password, "password2");
    }

    #[test]
    fn test_switch_active_user_user2_active() {
        let mut secret = VaultStructure {
            postgresql_active_user: "user2".to_string(),
            postgresql_active_user_password: "password2".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };

        switch_active_user(&mut secret);

        assert_eq!(secret.postgresql_active_user, "user1");
        assert_eq!(secret.postgresql_active_user_password, "password1");
    }

    #[test]
    fn test_update_passive_user_password_user1_active() {
        let mut secret = VaultStructure {
            postgresql_active_user: "user1".to_string(),
            postgresql_active_user_password: "password1".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };

        let new_password = "new_password".to_string();

        update_passive_user_password(&mut secret, new_password.clone());

        assert_eq!(secret.postgresql_active_user, "user1");
        assert_eq!(secret.postgresql_active_user_password, "password1");
        assert_eq!(secret.postgresql_user_2_password, new_password);
    }

    #[test]
    fn test_update_passive_user_password_user2_active() {
        let mut secret = VaultStructure {
            postgresql_active_user: "user2".to_string(),
            postgresql_active_user_password: "password2".to_string(),
            postgresql_user_1: "user1".to_string(),
            postgresql_user_1_password: "password1".to_string(),
            postgresql_user_2: "user2".to_string(),
            postgresql_user_2_password: "password2".to_string(),
        };

        let new_password = "new_password".to_string();

        update_passive_user_password(&mut secret, new_password.clone());

        assert_eq!(secret.postgresql_active_user, "user2");
        assert_eq!(secret.postgresql_active_user_password, "password2");
        assert_eq!(secret.postgresql_user_1_password, new_password);
    }
}
