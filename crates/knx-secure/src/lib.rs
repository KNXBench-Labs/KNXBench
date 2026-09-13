//! Isolated key material subsystem. Key material never enters the project
//! model, a report, an export, or a log.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// PBKDF2 iteration count fixed by the `.knxproj` password-protection
/// scheme: KNX Standard v3.0.0, Project Schema23 v01.00.00, clause 4.2.4
/// "Password protection", p.64/64.
const PBKDF2_ITERATIONS: u32 = 65_536;

/// The fixed salt for the same scheme, ASCII-encoded (the clause's own
/// wording calls this "little-endian UTF-8 without BOM", which for
/// pure-ASCII bytes is the same thing).
const PBKDF2_SALT: &[u8] = b"21.project.ets.knx.org";

/// Derived-key length in bytes for the same scheme.
const DERIVED_KEY_LEN: usize = 32;

/// The PBKDF2-derived ZIP-container password for a password-protected
/// `.knxproj`.
///
/// This is key material: it carries no `Display` impl, no `serde` impl,
/// and its `Debug` impl prints a fixed placeholder rather than the
/// password, so it cannot end up in a log line, an error message, or
/// anything a report serialises by any of the ordinary ways Rust code
/// prints or serialises a value. The one sanctioned way out is
/// [`ZipPassword::expose`], for the one call site that will eventually
/// hand this to a ZIP-decryption API — nothing in this crate calls it yet.
///
/// This type does *not* zero its buffer on drop. Doing that reliably needs
/// a volatile write (the `zeroize` crate, not currently a dependency of
/// this crate); a hand-rolled zero-on-drop without one is liable to be
/// optimised away by the compiler as a dead store, which would claim a
/// guarantee this code does not actually make. The guarantee this type
/// does make is the type-level one above, enforced at compile time.
pub struct ZipPassword(String);

impl ZipPassword {
    /// Returns the derived password's Base64 form. The caller becomes
    /// responsible for not logging, printing, or otherwise leaking it —
    /// this call is the boundary where that responsibility starts.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ZipPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ZipPassword(REDACTED)")
    }
}

/// Derives the ZIP-container password for a password-protected `.knxproj`
/// from the project password the user typed into ETS, per the scheme the
/// KNX Standard specifies for schema ≥ 21 (ETS6) projects:
///
/// KNX Standard v3.0.0, Project Schema23 v01.00.00, clause 4.2.4
/// "Password protection", p.64/64:
///
/// ```text
/// ZipPassword := Base64( PBKDF2( HMAC-SHA256, ProjectPassword,
///                                "21.project.ets.knx.org", 65536, 32 ) )
/// ```
///
/// The project password is encoded little-endian UTF-16 without a BOM
/// before hashing; the salt is encoded ASCII. Verified against all three
/// of the clause's own published test vectors — see this module's tests,
/// including the third, whose password characters had to be recovered
/// from a broken PDF text layer rather than read off directly.
///
/// This function only derives the password. It does not open, decrypt, or
/// even touch the encrypted ZIP entry itself — container decryption stays
/// refused (`ContainerError::PasswordProtected` in `knx-etsproj`) until a
/// real password-protected project exists to verify that half against.
///
/// `#[must_use]`: dropping the result silently discards the 65,536 PBKDF2
/// iterations that produced it. Note the asymmetry this leaves: `project_password`
/// is an ordinary `&str`, un-zeroed and free to be copied by whatever called
/// this function, while only the *output* gets the documented handling above
/// — the input's hygiene is the caller's problem, not this function's.
#[must_use]
pub fn derive_knxproj_zip_password(project_password: &str) -> ZipPassword {
    // `project_password.len()` is a UTF-8 byte count, not a UTF-16 code-unit
    // count, so `len() * 2` is not the right capacity: a 4-byte non-BMP
    // character needs 4 UTF-16LE bytes (one surrogate pair), not 8. Counting
    // the actual UTF-16 units first costs a second pass over a
    // password-length string, which is cheap, and gives an exact capacity
    // instead of a guess.
    let unit_count = project_password.encode_utf16().count();
    let mut utf16le: Vec<u8> = Vec::with_capacity(unit_count * 2);
    for unit in project_password.encode_utf16() {
        utf16le.extend_from_slice(&unit.to_le_bytes());
    }

    let mut derived = [0u8; DERIVED_KEY_LEN];
    pbkdf2_hmac::<Sha256>(&utf16le, PBKDF2_SALT, PBKDF2_ITERATIONS, &mut derived);

    let encoded = BASE64_STANDARD.encode(derived);

    ZipPassword(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Source for every vector in this module: KNX Standard v3.0.0, Project
    // Schema23 v01.00.00, clause 4.2.4 "Password protection", p.64/64:
    //
    //   "E.g., the project password a would result in the zip password
    //   string +FAwP4iI7/Pu4WB3HdIHbbFmteLahPAVkjJShKeozAA=, test in
    //   2+IIP7ErCPPKxFjJXc59GFx2+w/1VTLHjJ2duc04CYQ= and Penn¥w1se <?> in
    //   ZjlYlh+eTtoHvFadU7+EKvF4jOdEm7WkP49uanOMMk0=."
    //
    // These are the Standard's own vectors, not this implementation's —
    // an implementation asserting its own output back at itself would be
    // a defect, not a test.

    #[test]
    fn zip_password_matches_standard_vector_a() {
        let derived = derive_knxproj_zip_password("a");
        assert_eq!(
            derived.expose(),
            "+FAwP4iI7/Pu4WB3HdIHbbFmteLahPAVkjJShKeozAA="
        );
    }

    #[test]
    fn zip_password_matches_standard_vector_test() {
        let derived = derive_knxproj_zip_password("test");
        assert_eq!(
            derived.expose(),
            "2+IIP7ErCPPKxFjJXc59GFx2+w/1VTLHjJ2duc04CYQ="
        );
    }

    // The clause's third vector is a password containing non-ASCII
    // characters. Both the KNX spec corpus's Markdown extraction
    // (pdftotext-derived) and a direct `pdftotext` run on the source PDF
    // render it as "Penn¥w1se" followed by an unmappable glyph — neither
    // tool's ToUnicode CMap resolves the final character, so the exact
    // code points are not recoverable from either text extraction.
    //
    // Rendering page 64 to a 600 DPI raster (`pdftoppm -r 600`) and
    // inspecting the glyph directly, rather than trusting text
    // extraction, shows it unambiguously as the "Clown Face" emoji
    // (U+1F921 🤡) — red hair tufts, blue-ringed eyes, red nose, pink
    // smile, matching that emoji's standard glyph in every font that
    // ships it. The corpus's own Markdown extraction already shows a
    // literal space between "w1se" and the unmappable glyph (raw bytes:
    // w-1-s-e, then 0x20, then the U+FFFD replacement character), and
    // `pdftotext`'s per-glyph output (via `gs -sDEVICE=txtwrite`) is
    // consistent with that: the visible password is bounded by ordinary
    // word-spacing on both sides, same as the "a" and "test" vectors above.
    // That is not, by itself, proof the space is a real character rather
    // than a layout artifact — typesetting can insert space before a
    // wide inline glyph purely for layout — so it was still worth
    // resolving independently rather than taking the extracted space on
    // faith.
    //
    // That residual ambiguity was resolved, not guessed: both candidates
    // ("Penn\u{a5}w1se\u{1f921}" and "Penn\u{a5}w1se \u{1f921}") were run
    // through the derivation this module implements — the same function
    // already verified correct against the two vectors above — and only
    // one, "Penn¥w1se 🤡" (with the space), reproduces the clause's
    // published hash `ZjlYlh+eTtoHvFadU7+EKvF4jOdEm7WkP49uanOMMk0=`
    // exactly. Every other candidate this module was tried against does
    // not.
    //
    // This is corroboration, not circularity: PBKDF2-HMAC-SHA256 is not
    // invertible, so matching the Standard's independently published hash
    // is not something a wrong reconstruction could do by chance. It is,
    // however, a different evidence path than plain text extraction, and
    // this repository's task brief anticipated this vector would likely
    // stay unrecoverable — so this test is included with that full
    // account attached, for a human to accept or reject on its merits,
    // rather than silently promoted to the same footing as the two
    // vectors above.
    #[test]
    fn zip_password_matches_standard_vector_pennywise_reconstructed_from_pdf_render() {
        let derived = derive_knxproj_zip_password("Penn\u{a5}w1se \u{1f921}");
        assert_eq!(
            derived.expose(),
            "ZjlYlh+eTtoHvFadU7+EKvF4jOdEm7WkP49uanOMMk0="
        );
    }

    #[test]
    fn debug_impl_redacts_the_password() {
        let derived = derive_knxproj_zip_password("a");
        assert_eq!(format!("{derived:?}"), "ZipPassword(REDACTED)");
    }
}
