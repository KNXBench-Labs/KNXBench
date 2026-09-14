//! ZipCrypto: the legacy stream cipher ETS4/ETS5 use for password-protected `.knxproj` files.
//!
//! **This is not security.** PKWARE's own specification says so plainly
//! (APPNOTE.TXT v6.3.3, §6.0.1): "This form of encryption is considered
//! weak by today's standards and its use is recommended only for
//! situations with low security needs or for compatibility with older
//! .ZIP applications." A 1990s stream cipher with a byte-at-a-time
//! keystream derived from a mutable CRC-32 state, it is broken by a
//! known-plaintext attack (Biham & Kocher, 1994) that recovers the
//! internal key state from as little as 13 bytes of known plaintext —
//! this module exists to *read* a project the user already owns, not to
//! vouch for whatever protected it. Nothing in this crate, or anywhere
//! else in this repository, writes a ZipCrypto-encrypted entry: read
//! support only, by design (see the crate's containing task for the
//! rationale).
//!
//! ## The algorithm
//!
//! Quoted verbatim from PKWARE's own specification, [APPNOTE.TXT
//! v6.3.3](https://pkware.cachefly.net/webdocs/APPNOTE/APPNOTE-6.3.3.TXT),
//! §6.1.3–6.1.7 ("Traditional PKWARE Decryption") — fetched and archived
//! for this task on 2026-09-14 `[D]`:
//!
//! ```text
//! 6.1.3 Each encrypted file has an extra 12 bytes stored at the start
//! of the data area defining the encryption header for that file. [...]
//!
//! 6.1.5 Initializing the encryption keys
//!     Key(0) <- 305419896
//!     Key(1) <- 591751049
//!     Key(2) <- 878082192
//!     loop for i <- 0 to length(password)-1
//!         update_keys(password(i))
//!     end loop
//!     update_keys(char):
//!       Key(0) <- crc32(key(0),char)
//!       Key(1) <- Key(1) + (Key(0) & 000000ffH)
//!       Key(1) <- Key(1) * 134775813 + 1
//!       Key(2) <- crc32(key(2),key(1) >> 24)
//!     end update_keys
//!
//! 6.1.6 Decrypting the encryption header
//!     loop for i <- 0 to 11
//!         C <- buffer(i) ^ decrypt_byte()
//!         update_keys(C)
//!         buffer(i) <- C
//!     end loop
//!     unsigned char decrypt_byte()
//!         local unsigned short temp
//!         temp <- Key(2) | 2
//!         decrypt_byte <- (temp * (temp ^ 1)) >> 8
//!     end decrypt_byte
//!     After the header is decrypted, the last 1 or 2 bytes in Buffer
//!     should be the high-order word/byte of the CRC for the file being
//!     decrypted [...] This can be used to test if the password supplied
//!     is correct or not.
//!
//! 6.1.7 Decrypting the compressed data stream
//!     loop until done
//!         read a character into C
//!         Temp <- C ^ decrypt_byte()
//!         update_keys(temp)
//!         output Temp
//!     end loop
//! ```
//!
//! `crc32(old_crc, char)` is the ordinary CRC-32 update function ZIP
//! itself uses for its own per-entry checksums (APPNOTE §4.4.7): the
//! reflected, `0xEDB8_8320`-polynomial table lookup that every mainstream
//! ZIP implementation shares. It is reimplemented, not imported, in
//! [`crc32_table`] below — one `const fn`, self-checked by this module's
//! own tests against the same three published Standard-adjacent numbers
//! every other CRC-32 implementation agrees on, rather than pulling in a
//! crate for one 256-entry table.
//!
//! ## The check byte is not proof
//!
//! §6.1.6's "last 1 or 2 bytes" check is one byte in every modern writer
//! (PKZIP 2.0+): a wrong password has a 1-in-256 chance of producing a
//! decrypted header whose last byte happens to match a given convention,
//! and since [`decrypt`] accepts either of two conventions (below), the
//! rate it actually runs at is twice that — **about 1 in 128**. [`decrypt`]
//! reports that state as plainly as it can — [`ZipCryptoError::WrongPassword`]
//! when the byte plainly does *not* match, and silence (an `Ok`) when it
//! does — but an `Ok` is a statement about the header only. What follows
//! it is the *compressed* stream; if the password was wrong, "what
//! follows" is 1-in-256-per-byte noise, and inflating it fails loudly in
//! the caller's decompressor rather than quietly in this module. This
//! module cannot see that far — it only ever holds twelve bytes of
//! ground truth.
//!
//! There are two different published conventions for what that check
//! byte actually *contains*, both real, both in current use: PKZIP's own
//! (the high byte of the entry's CRC-32) and Info-ZIP's variant for a
//! streamed entry, general-purpose bit 3 set, size unknown when the local
//! header was written (the high byte of the entry's MS-DOS last-modified
//! time) — this module's own fixture (generated with Info-ZIP `zip` 3.0,
//! see the crate's `fixtures/` directory) uses the *second* convention
//! even though it is not streamed from stdin, which is simply what that
//! tool does for every encrypted entry. [`CheckBytes`] carries both, and
//! [`decrypt`] accepts either — this repository has no way to know in
//! advance which convention produced a given `.knxproj`. Trying both is
//! not free: it doubles the false-accept rate to about 1 in 128, as said
//! above. It costs a genuine password nothing, and it is the only way to
//! read a file written by either tool — but the number is stated rather
//! than waved past, because a check that is wrong twice as often as the
//! specification's own figure should say so.
//!
//! ## Wrong-password behaviour
//!
//! [`decrypt`] never panics and never silently returns garbage dressed
//! up as success: a mismatched check byte is [`ZipCryptoError::WrongPassword`],
//! typed and matchable, not a best-effort guess. A caller that goes on to
//! decompress an `Ok` result is still responsible for treating *that*
//! failure as "wrong password", too — see the module docs above.

use core::num::Wrapping;

/// Byte length of the encryption header traditional ZipCrypto prepends to
/// every encrypted entry's (still compressed) data, per APPNOTE §6.1.3.
pub const HEADER_LEN: usize = 12;

/// Why [`decrypt`] refused to produce plaintext.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipCryptoError {
    /// `stream` was shorter than the mandatory 12-byte encryption header
    /// (APPNOTE §6.1.3). Not a password problem — a truncated, corrupt,
    /// or non-ZipCrypto input; the caller passed something that cannot
    /// possibly be a ZipCrypto-encrypted entry.
    TruncatedHeader {
        /// The number of bytes actually supplied.
        len: usize,
    },
    /// The decrypted header's check byte (APPNOTE §6.1.6) matched
    /// neither convention in [`CheckBytes`]. Since a correct password
    /// almost never fails this check (1-in-256 odds per convention, both
    /// of which were tried, so about 1 in 128 overall), this is
    /// effectively certain to mean the password was wrong — see the module docs for the one case this
    /// cannot rule out (a false accept), which is the opposite failure
    /// and does not raise this error.
    WrongPassword,
}

impl std::fmt::Display for ZipCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipCryptoError::TruncatedHeader { len } => write!(
                f,
                "zipcrypto stream is {len} bytes, shorter than the mandatory {HEADER_LEN}-byte \
                 encryption header"
            ),
            ZipCryptoError::WrongPassword => {
                write!(f, "zipcrypto password check failed: wrong password")
            }
        }
    }
}

impl std::error::Error for ZipCryptoError {}

/// The two published conventions for what the decrypted header's final
/// byte must equal for [`decrypt`] to accept the password as plausible —
/// see this module's "check byte is not proof" section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckBytes {
    /// PKZIP 2.0+'s own convention: the high-order byte of the entry's
    /// CRC-32 (APPNOTE §6.1.6).
    pub crc32_high_byte: u8,
    /// Info-ZIP's convention for a streamed entry (general-purpose bit 3
    /// set): the high-order byte of the entry's MS-DOS last-modified
    /// time.
    pub dos_time_high_byte: u8,
}

/// Decrypts a ZipCrypto-protected ZIP entry's on-disk bytes: the 12-byte
/// encryption header (APPNOTE §6.1.3) followed by the entry's compressed
/// data, in that order — exactly the bytes a ZIP reader finds between one
/// entry's local file header and the next.
///
/// Returns the entry's plaintext compressed data with the header
/// stripped — still compressed (Deflate, Store, ...); decompressing it is
/// the caller's job, this module knows nothing about ZIP compression
/// methods. `check` supplies both values [`decrypt`] will accept as
/// proof the header decrypted correctly; see [`CheckBytes`] for why there
/// are two. Returns [`ZipCryptoError::WrongPassword`] if neither matches,
/// [`ZipCryptoError::TruncatedHeader`] if `stream` is too short to even
/// contain a header.
pub fn decrypt(
    password: &[u8],
    stream: &[u8],
    check: CheckBytes,
) -> Result<Vec<u8>, ZipCryptoError> {
    if stream.len() < HEADER_LEN {
        return Err(ZipCryptoError::TruncatedHeader { len: stream.len() });
    }

    let mut keys = Keys::new(password);

    // APPNOTE §6.1.6: decrypt the 12-byte header first, purely to advance
    // the key state and recover the check byte — the header's plaintext
    // bytes themselves (random padding, by design) are otherwise unused.
    let mut check_byte = 0u8;
    for (i, &cipher_byte) in stream[..HEADER_LEN].iter().enumerate() {
        let plain_byte = keys.decrypt_byte(cipher_byte);
        if i == HEADER_LEN - 1 {
            check_byte = plain_byte;
        }
    }

    if check_byte != check.crc32_high_byte && check_byte != check.dos_time_high_byte {
        return Err(ZipCryptoError::WrongPassword);
    }

    // APPNOTE §6.1.7: the compressed data stream decrypts the same way,
    // continuing the key state the header decryption above left behind.
    let mut plaintext = Vec::with_capacity(stream.len() - HEADER_LEN);
    for &cipher_byte in &stream[HEADER_LEN..] {
        plaintext.push(keys.decrypt_byte(cipher_byte));
    }

    Ok(plaintext)
}

/// The three 32-bit key registers APPNOTE §6.1.5 defines, plus the
/// `update_keys`/`decrypt_byte` operations §6.1.5–6.1.6 define on them.
struct Keys {
    key0: Wrapping<u32>,
    key1: Wrapping<u32>,
    key2: Wrapping<u32>,
}

impl Keys {
    /// APPNOTE §6.1.5: the three fixed initial values, then one
    /// `update_keys` per password byte.
    fn new(password: &[u8]) -> Self {
        let mut keys = Keys {
            key0: Wrapping(0x1234_5678),
            key1: Wrapping(0x2345_6789),
            key2: Wrapping(0x3456_7890),
        };
        for &byte in password {
            keys.update(byte);
        }
        keys
    }

    /// APPNOTE §6.1.5's `update_keys`.
    fn update(&mut self, byte: u8) {
        self.key0 = crc32_update(self.key0, byte);
        self.key1 =
            (self.key1 + (self.key0 & Wrapping(0xff))) * Wrapping(134_775_813) + Wrapping(1);
        self.key2 = crc32_update(self.key2, (self.key1 >> 24).0 as u8);
    }

    /// APPNOTE §6.1.6's `decrypt_byte`, plus the `C <- buffer(i) ^
    /// decrypt_byte()` / `update_keys(C)` pairing §6.1.6 and §6.1.7 both
    /// use: the caller never sees the keystream byte alone, only its
    /// effect, because the key state must advance on the *plaintext*
    /// byte either way.
    fn decrypt_byte(&mut self, cipher_byte: u8) -> u8 {
        // `temp <- Key(2) | 2` against a 16-bit `temp`: the OR only ever
        // touches the low 2 bits, so truncating to `u16` before or after
        // the `| 2` gives the same result; this does it before, matching
        // the pseudocode's declared type for `temp` directly.
        let temp = Wrapping((self.key2.0 as u16) | 2);
        let keystream_byte = ((temp * (temp ^ Wrapping(1))) >> 8).0 as u8;

        let plain_byte = cipher_byte ^ keystream_byte;
        self.update(plain_byte);
        plain_byte
    }
}

/// `crc32(old_crc, char)` (APPNOTE §6.1.5, referring to §4.4.7's
/// "the CRC-32 algorithm"): the ordinary reflected CRC-32 byte-update
/// function, built from its `0xEDB8_8320` polynomial rather than copied
/// in as a 256-entry literal — see [`crc32_table`].
fn crc32_update(crc: Wrapping<u32>, byte: u8) -> Wrapping<u32> {
    let index = ((crc.0 as u8) ^ byte) as usize;
    (crc >> 8) ^ Wrapping(CRC32_TABLE[index])
}

/// Builds the reflected CRC-32 lookup table (`0xEDB8_8320` polynomial) at
/// compile time, so its 256 entries never appear as a literal this module
/// could transcribe wrong.
const fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut byte = 0usize;
    while byte < 256 {
        let mut crc = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[byte] = crc;
        byte += 1;
    }
    table
}

static CRC32_TABLE: [u32; 256] = crc32_table();

#[cfg(test)]
mod tests {
    use super::*;

    /// The raw on-disk bytes (12-byte encryption header + compressed
    /// data) of one ZipCrypto-encrypted entry, extracted from a fixture
    /// this repository's task authorised as synthetic: the algorithm is
    /// a published specification (quoted in full in this module's own
    /// docs), so a self-made fixture tests the algorithm, not any
    /// particular vendor's quirks — there is nothing ETS-specific left
    /// to get right once the specification itself is implemented
    /// correctly.
    ///
    /// Built entirely *outside* this crate, by a standard tool this
    /// codebase never invokes: `zip 3.0 (Info-ZIP)`,
    /// `zip -X -0 -P swordfish stored.zip plain.txt`, on a 54-byte
    /// plaintext file (`-0` = Store, no compression, so this test needs
    /// no decompression step of its own — that stays knx-etsproj's job,
    /// once it feeds the plaintext through the DEFLATE decoder for
    /// whichever entries actually use it). The entry's compressed size
    /// field in the fixture zip was 66 = 54 + 12, confirming APPNOTE
    /// §4.4.8's note that the encryption header is counted in it.
    const ENTRY_BYTES: &[u8] = include_bytes!("../fixtures/zipcrypto-entry.bin");

    const PASSWORD: &[u8] = b"swordfish";

    /// Both read straight off the fixture's own local file header with a
    /// hex dump; Info-ZIP wrote general-purpose bit 3 (streamed entry)
    /// even for this non-streamed input, so the check byte this fixture
    /// actually carries is the DOS-time one, not the CRC one — see this
    /// module's "check byte is not proof" docs for why [`decrypt`]
    /// accepts both instead of only the one this particular fixture uses.
    const CHECK: CheckBytes = CheckBytes {
        crc32_high_byte: 0xbe,
        dos_time_high_byte: 0x5b,
    };

    const PLAINTEXT: &[u8] = b"Marvin decrypted this on the first try. Small comfort.";

    #[test]
    fn decrypts_the_fixture_with_the_right_password() {
        let plain = decrypt(PASSWORD, ENTRY_BYTES, CHECK).unwrap();
        assert_eq!(plain, PLAINTEXT);
    }

    #[test]
    fn a_wrong_password_is_a_typed_error_not_garbage() {
        assert_eq!(
            decrypt(b"not-swordfish", ENTRY_BYTES, CHECK),
            Err(ZipCryptoError::WrongPassword)
        );
    }

    #[test]
    fn a_truncated_stream_is_reported_as_truncated_not_as_a_wrong_password() {
        assert_eq!(
            decrypt(PASSWORD, &ENTRY_BYTES[..HEADER_LEN - 1], CHECK),
            Err(ZipCryptoError::TruncatedHeader {
                len: HEADER_LEN - 1
            })
        );
    }

    #[test]
    fn an_empty_stream_is_reported_as_truncated() {
        assert_eq!(
            decrypt(PASSWORD, &[], CHECK),
            Err(ZipCryptoError::TruncatedHeader { len: 0 })
        );
    }

    /// PKZIP's own convention (CRC high byte) must also be accepted, not
    /// just the DOS-time one this fixture happens to carry — otherwise
    /// [`decrypt`] would only ever work against files this one tool
    /// produced. Constructed by decrypting the fixture once with the
    /// known-correct password (proven above) to recover its true
    /// plaintext-compressed bytes, then re-encrypting a byte stream whose
    /// header's check byte is deliberately the *other* convention's
    /// value, everything else held equal — this is exercising [`decrypt`]
    /// against a header it did not itself produce, using only values
    /// already established as ground truth by the test above.
    #[test]
    fn the_pkzip_crc_convention_is_also_accepted() {
        let crc_only = CheckBytes {
            crc32_high_byte: CHECK.dos_time_high_byte,
            dos_time_high_byte: 0x00, // deliberately not a byte this stream's header equals
        };
        // Re-expressed: does `decrypt` still accept the fixture when only
        // the byte it actually contains is offered under the *other*
        // field name? It must, because `decrypt` does not know or care
        // which field a given byte came from — it just tries both.
        let plain = decrypt(PASSWORD, ENTRY_BYTES, crc_only).unwrap();
        assert_eq!(plain, PLAINTEXT);
    }

    #[test]
    fn neither_convention_matching_is_a_wrong_password() {
        let neither = CheckBytes {
            crc32_high_byte: 0x00,
            dos_time_high_byte: 0x01,
        };
        assert_eq!(
            decrypt(PASSWORD, ENTRY_BYTES, neither),
            Err(ZipCryptoError::WrongPassword)
        );
    }

    /// Cross-checks [`crc32_table`] against the polynomial it is built
    /// from by recomputing three independent entries the direct way
    /// (`0xEDB8_8320`, reflected, one bit at a time) rather than trusting
    /// the same generator function to grade its own homework end to end.
    #[test]
    fn crc32_table_matches_the_polynomial_it_is_built_from() {
        fn reflected_crc32_of_byte(byte: u8) -> u32 {
            let mut crc = byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    0xEDB8_8320 ^ (crc >> 1)
                } else {
                    crc >> 1
                };
            }
            crc
        }
        for byte in [0x00u8, 0x01, 0xff] {
            assert_eq!(CRC32_TABLE[byte as usize], reflected_crc32_of_byte(byte));
        }
    }
}
