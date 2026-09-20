//! PBKDF2-HMAC-SHA256 password hashing and constant-time verification for the server's login.

use base64::engine::general_purpose::STANDARD_NO_PAD as B64;
use base64::Engine as _;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// OWASP's 2023 *Password Storage Cheat Sheet* floor for PBKDF2-HMAC-SHA256
/// ("PBKDF2-HMAC-SHA256: 600,000 iterations"), and therefore this server's
/// default. It is a floor, not a target: the count is stored in the hash
/// string, so raising it later needs no migration — an old hash keeps
/// verifying with the count it was made with, and the next
/// `--hash-password` produces the new one.
pub const DEFAULT_ITERATIONS: u32 = 600_000;

/// 16 bytes of salt, the length the same cheat sheet asks for. Its only
/// job is to make one precomputed table per password useless; it is not
/// secret and is stored beside the hash.
const SALT_LEN: usize = 16;

/// 32 bytes of output — SHA-256's own block output, so asking PBKDF2 for
/// more would cost a second full iteration chain for no added strength.
const HASH_LEN: usize = 32;

/// The algorithm identifier, in PHC string format's `$<id>$` position.
const ALGORITHM: &str = "pbkdf2-sha256";

/// A parsed stored credential: the parameters a verification needs, and
/// the digest it must reproduce.
///
/// The serialised form is PHC-shaped and self-describing, so a hash made
/// by one build still verifies under the next one even if the defaults
/// above move:
///
/// ```text
/// $pbkdf2-sha256$i=600000$<salt>$<hash>
/// ```
///
/// `<salt>` and `<hash>` are standard base64 (RFC 4648 §4) with the
/// padding stripped, as PHC's own "B64" alphabet prescribes. Nothing here
/// is secret except the password that produced it: the whole string is
/// meant to be pasted into `KNX_AUTH_PASSWORD_HASH`, a compose file or a
/// secret store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPassword {
    iterations: u32,
    salt: Vec<u8>,
    hash: Vec<u8>,
}

impl StoredPassword {
    /// Derives a credential from a plaintext password and an explicit
    /// salt. Private because a caller who chooses the salt can repeat it;
    /// [`hash_password`] is the door everyone else uses.
    fn derive(password: &str, salt: Vec<u8>, iterations: u32) -> Self {
        let mut hash = vec![0u8; HASH_LEN];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut hash);
        Self {
            iterations,
            salt,
            hash,
        }
    }

    /// Reads a stored string back. Every failure is a deployment mistake
    /// worth naming precisely, because the operator sees this message at
    /// startup and has to fix the environment variable behind it.
    pub fn parse(stored: &str) -> Result<Self, String> {
        if stored.is_empty() {
            return Err("password hash is empty".to_string());
        }
        let mut parts = stored.split('$');
        if parts.next() != Some("") {
            return Err("password hash must start with `$`".to_string());
        }
        match parts.next() {
            Some(ALGORITHM) => {}
            Some(other) => {
                return Err(format!(
                    "unsupported password hash algorithm `{other}`, expected `{ALGORITHM}`"
                ))
            }
            None => return Err("password hash names no algorithm".to_string()),
        }
        let iterations = parts
            .next()
            .ok_or_else(|| "password hash carries no iteration count".to_string())?
            .strip_prefix("i=")
            .ok_or_else(|| "password hash iteration field must read `i=<count>`".to_string())?
            .parse::<u32>()
            .map_err(|e| format!("password hash iteration count is not a number: {e}"))?;
        if iterations == 0 {
            return Err("password hash iteration count must be at least 1".to_string());
        }
        let salt = decode_field(parts.next(), "salt")?;
        let hash = decode_field(parts.next(), "hash")?;
        if parts.next().is_some() {
            return Err("password hash has more fields than the format defines".to_string());
        }
        if salt.is_empty() {
            return Err("password hash carries an empty salt".to_string());
        }
        if hash.is_empty() {
            return Err("password hash carries an empty digest".to_string());
        }
        Ok(Self {
            iterations,
            salt,
            hash,
        })
    }

    /// How many iterations this credential was made with — read at startup
    /// so a deliberately cheap hash can be warned about rather than
    /// silently accepted as if it were [`DEFAULT_ITERATIONS`].
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Recomputes the digest from `password` under this credential's own
    /// stored parameters and compares the two in constant time.
    ///
    /// Deliberately blocking and deliberately slow — that is the entire
    /// point of the iteration count. Call it from `spawn_blocking`, never
    /// straight from an async handler.
    pub fn verify(&self, password: &str) -> bool {
        let mut derived = vec![0u8; self.hash.len()];
        pbkdf2_hmac::<Sha256>(
            password.as_bytes(),
            &self.salt,
            self.iterations,
            &mut derived,
        );
        constant_time_eq(&derived, &self.hash)
    }
}

impl std::fmt::Display for StoredPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "${ALGORITHM}$i={}${}${}",
            self.iterations,
            B64.encode(&self.salt),
            B64.encode(&self.hash)
        )
    }
}

fn decode_field(field: Option<&str>, name: &str) -> Result<Vec<u8>, String> {
    let field = field.ok_or_else(|| format!("password hash carries no {name}"))?;
    B64.decode(field)
        .map_err(|e| format!("password hash {name} is not unpadded base64: {e}"))
}

/// Compares two byte strings without an early exit, so the time taken says
/// nothing about *where* they first differ.
///
/// The lengths are compared first and that comparison does leak: both
/// operands here are fixed-width digests, so their length is public
/// information already. The loop itself has no `break` and no branch on
/// the data — it accumulates the difference and inspects it once.
///
/// That shape is best-effort, not a guarantee. Rust and LLVM promise
/// nothing about preserving it, and a sufficiently clever optimiser is
/// entitled to reintroduce an early exit; [`std::hint::black_box`] is the
/// strongest discouragement available without inline assembly or a crate
/// like `subtle`, and it is a hint. It is enough here because of what this
/// function is given: both operands are PBKDF2 outputs over a salt the
/// caller cannot choose, so an attacker cannot steer the bytes being
/// compared and has nothing to learn from where they first differ.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut difference = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        difference |= x ^ y;
    }
    std::hint::black_box(difference) == 0
}

/// Hashes `password` at [`DEFAULT_ITERATIONS`] with a fresh random salt.
/// This is what `--hash-password` and the `KNX_AUTH_PASSWORD` convenience
/// path both call.
pub fn hash_password(password: &str) -> Result<String, String> {
    hash_password_with_iterations(password, DEFAULT_ITERATIONS)
}

/// [`hash_password`] with the work factor spelled out.
///
/// It exists for the tests: at the production count a single verification
/// costs seconds in an unoptimised build, so a suite that logs in a dozen
/// times would spend a minute proving nothing about iteration counts.
///
/// No command-line path reaches it — `--hash-password` always hashes at
/// [`DEFAULT_ITERATIONS`], deliberately, because a flag for lowering the
/// work factor is a flag someone will lower it with. A hash made elsewhere
/// at a lower count still verifies (the count travels inside the string)
/// and is reported at startup as the compromise it is.
pub fn hash_password_with_iterations(password: &str, iterations: u32) -> Result<String, String> {
    let mut salt = vec![0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|e| format!("failed to read random salt: {e}"))?;
    Ok(StoredPassword::derive(password, salt, iterations).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cheap enough to use freely in tests and obviously not a production
    /// number, so nothing here can be mistaken for an endorsement of it.
    const TEST_ITERATIONS: u32 = 1_000;

    #[test]
    fn a_hash_round_trips_through_its_stored_form() {
        let stored = hash_password_with_iterations("correct horse", TEST_ITERATIONS).unwrap();
        assert!(stored.starts_with("$pbkdf2-sha256$i=1000$"));
        let parsed = StoredPassword::parse(&stored).unwrap();
        assert_eq!(parsed.iterations(), TEST_ITERATIONS);
        assert!(parsed.verify("correct horse"));
        assert_eq!(parsed.to_string(), stored);
    }

    #[test]
    fn verification_rejects_a_near_miss() {
        let stored = hash_password_with_iterations("correct horse", TEST_ITERATIONS).unwrap();
        let parsed = StoredPassword::parse(&stored).unwrap();
        for near_miss in [
            "correct hors",
            "correct horsf",
            "correct horse ",
            " correct horse",
            "Correct horse",
            "",
        ] {
            assert!(
                !parsed.verify(near_miss),
                "`{near_miss}` must not verify against `correct horse`"
            );
        }
    }

    #[test]
    fn the_same_password_hashes_differently_every_time() {
        let a = hash_password_with_iterations("same", TEST_ITERATIONS).unwrap();
        let b = hash_password_with_iterations("same", TEST_ITERATIONS).unwrap();
        assert_ne!(a, b, "a fresh salt must make the stored strings differ");
        assert!(StoredPassword::parse(&a).unwrap().verify("same"));
        assert!(StoredPassword::parse(&b).unwrap().verify("same"));
    }

    /// The one test that pays the production work factor, because a
    /// default nothing ever exercises is a default nobody can trust. One
    /// derivation and one verification, roughly seven seconds in an
    /// unoptimised build; every other test above uses [`TEST_ITERATIONS`].
    #[test]
    fn the_default_work_factor_is_owasps_and_it_verifies() {
        assert_eq!(DEFAULT_ITERATIONS, 600_000);
        let stored = hash_password("hunter2 has seen things").unwrap();
        assert!(stored.starts_with("$pbkdf2-sha256$i=600000$"));
        assert!(StoredPassword::parse(&stored)
            .unwrap()
            .verify("hunter2 has seen things"));
    }

    #[test]
    fn a_stored_hash_is_rejected_when_it_is_malformed() {
        for (bad, expected) in [
            ("", "password hash is empty"),
            ("pbkdf2-sha256$i=1$c2FsdA$aGFzaA", "must start with `$`"),
            (
                "$argon2id$i=1$c2FsdA$aGFzaA",
                "unsupported password hash algorithm",
            ),
            (
                "$pbkdf2-sha256$600000$c2FsdA$aGFzaA",
                "must read `i=<count>`",
            ),
            ("$pbkdf2-sha256$i=lots$c2FsdA$aGFzaA", "is not a number"),
            ("$pbkdf2-sha256$i=0$c2FsdA$aGFzaA", "at least 1"),
            ("$pbkdf2-sha256$i=1$c2FsdA", "carries no hash"),
            ("$pbkdf2-sha256$i=1$$aGFzaA", "empty salt"),
            ("$pbkdf2-sha256$i=1$c2FsdA$", "empty digest"),
            ("$pbkdf2-sha256$i=1$c2Fs!A$aGFzaA", "not unpadded base64"),
            ("$pbkdf2-sha256$i=1$c2FsdA$aGFzaA$extra", "more fields"),
        ] {
            let error = StoredPassword::parse(bad)
                .expect_err(&format!("`{bad}` must not parse as a stored hash"));
            assert!(
                error.contains(expected),
                "parsing `{bad}` should complain about `{expected}`, said `{error}`"
            );
        }
    }

    #[test]
    fn padded_base64_is_not_accepted_either() {
        // `c2FsdA==` is the padded spelling of the same salt. PHC's B64
        // alphabet has no padding, and quietly accepting both spellings
        // would mean one credential has two stored forms.
        assert!(StoredPassword::parse("$pbkdf2-sha256$i=1$c2FsdA==$aGFzaA").is_err());
    }

    #[test]
    fn constant_time_eq_agrees_with_ordinary_equality() {
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"Abc"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"abcd", b"abc"));
    }
}
