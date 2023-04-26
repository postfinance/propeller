use rand::distributions::{Alphanumeric, DistString};

use crate::CLI_ARGS;

pub(crate) fn generate_username(prefix: &str, length: usize) -> String {
    let random_part = Alphanumeric.sample_string(&mut rand::thread_rng(), length);

    let username = format!("{}{}", prefix, random_part);

    if CLI_ARGS.debug || CLI_ARGS.verbose {
        println!("🔎 Generated random username: {}", username);
    }

    username
}

/**
 * **Note:** In principle, all RNGs in Rand implementing CryptoRng are suitable as a source of
 * randomness for generating passwords (if they are properly seeded), but it is more conservative to
 * only use randomness directly from the operating system via the getrandom crate, or the
 * corresponding bindings of a crypto library.
 *
 * Source: https://rust-random.github.io/rand/rand/distributions/struct.Alphanumeric.html#passwords.
 */
pub(crate) fn generate_random_password(length: usize) -> String {
    let password = Alphanumeric.sample_string(&mut rand::thread_rng(), length);

    if CLI_ARGS.verbose {
        println!("👀 Generated random password: {}", password);
    }

    password
}
