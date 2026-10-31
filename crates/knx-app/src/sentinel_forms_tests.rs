//! Every encoding a lazy serializer might give a secret byte run, shared by App and Server tests.
//!
//! Included by `#[path]` from both crates' tests, so it depends on `std` only.

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// Every sentinel byte run the projection must never carry, in all encodings a lazy
/// serializer might pick: hex in both cases and separators, `0x`, decimal lists, base64
/// (every alignment, interior characters only) and the raw octets.
pub fn forbidden_forms(bytes: &[u8]) -> Vec<Vec<u8>> {
    let join = |sep: &str, f: &dyn Fn(u8) -> String| {
        bytes.iter().map(|b| f(*b)).collect::<Vec<_>>().join(sep)
    };
    let mut forms: Vec<String> = vec![hex(bytes), hex(bytes).to_uppercase()];
    for sep in [" ", ":", "-", ",", ", ", "."] {
        forms.push(join(sep, &|b| format!("{b:02x}")));
        forms.push(join(sep, &|b| format!("{b:02X}")));
        forms.push(join(sep, &|b| format!("0x{b:02x}")));
        forms.push(join(sep, &|b| b.to_string()));
    }
    forms.push(format!("[{}]", join(",", &|b| b.to_string())));
    forms.push(format!("[{}]", join(", ", &|b| b.to_string())));
    let mut out: Vec<Vec<u8>> = forms.into_iter().map(String::into_bytes).collect();
    for pad in 0..3 {
        let mut padded = vec![0u8; pad];
        padded.extend_from_slice(bytes);
        for alphabet in [B64_STD, B64_URL] {
            let text = base64(&padded, alphabet);
            let trimmed = text.trim_end_matches('=');
            // Drop the characters that mix in padding or the neighbour; keep the interior.
            let interior = &trimmed[(pad * 4).div_ceil(3).min(trimmed.len())..];
            if interior.len() > 2 {
                out.push(interior.as_bytes()[..interior.len() - 1].to_vec());
            }
        }
    }
    out.push(bytes.to_vec());
    out
}
const B64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn base64(bytes: &[u8], alphabet: &[u8; 64]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(alphabet[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}
pub fn assert_no_sentinel_bytes(label: &str, haystack: &[u8], sentinels: &[&[u8]]) {
    for run in sentinels {
        for form in forbidden_forms(run) {
            assert!(
                !haystack.windows(form.len()).any(|w| w == form.as_slice()),
                "{label}: leaked {:?}",
                String::from_utf8_lossy(&form)
            );
        }
    }
}
