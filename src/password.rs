use log::trace;
use rand::distr::Alphanumeric;
use rand::{thread_rng, Rng};

pub(crate) fn generate_random_password(length: usize) -> String {
    trace!("Generating random password");

    let mut rng = thread_rng();
    let password: String = (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();
    password
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_length() {
        for length in vec![8, 16, 32] {
            let password = generate_random_password(length);
            assert_eq!(password.len(), length);
        }
    }

    #[test]
    fn password_content() {
        let password = generate_random_password(10);
        assert!(password.chars().all(|c| c.is_alphanumeric()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_uppercase()));
    }
}
