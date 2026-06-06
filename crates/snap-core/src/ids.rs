//! Identifier and token generation.
//!
//! Photos are keyed by a ULID (time-ordered, lexicographically sortable) and
//! additionally carry an unguessable download token used in QR / share URLs,
//! so that knowing one photo's URL never reveals another's.

use rand::Rng;

/// Generate a new ULID string (26 chars, Crockford base32).
pub fn new_ulid() -> String {
    ulid::Ulid::new().to_string()
}

/// Generate an unguessable, URL-safe share/download token (~143 bits).
pub fn new_token() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..24)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulids_are_unique_and_sized() {
        let a = new_ulid();
        let b = new_ulid();
        assert_eq!(a.len(), 26);
        assert_ne!(a, b);
    }

    #[test]
    fn tokens_are_unguessable_length() {
        let t = new_token();
        assert_eq!(t.len(), 24);
        assert_ne!(new_token(), new_token());
    }
}
