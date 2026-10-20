//! Bounded JSON and deterministic ZIP serialization tests.
use super::*;

#[test]
fn output_writer_admits_64_mib_then_preserves_prefix_on_refusal() {
    let mut out = Bounded::new(67_108_864);
    let block = vec![b'x'; 1024 * 1024];
    for _ in 0..64 {
        out.write_all(&block).unwrap();
    }
    assert_eq!(out.cursor.get_ref().len(), 67_108_864);
    assert!(out.write_all(b"y").is_err());
    assert_eq!(out.cursor.get_ref().len(), 67_108_864);
    assert_eq!(out.cursor.get_ref().last(), Some(&b'x'));
    // ZipWriter seeks backwards to amend headers; rewriting admitted bytes is
    // allowed, seeking/writing beyond the final limit is not.
    out.seek(SeekFrom::Start(0)).unwrap();
    out.write_all(b"z").unwrap();
    let old = out.cursor.position();
    assert!(out.seek(SeekFrom::Start(67_108_865)).is_err());
    assert_eq!(out.cursor.position(), old);
}
#[test]
fn escaped_json_uses_the_bounded_writer_not_an_unlimited_buffer() {
    let value = "\0".repeat(11_184_811);
    let err = match json(&value) {
        Err(e) => e,
        Ok(_) => panic!("escaped JSON must exceed 64 MiB"),
    };
    assert!(err.contains("output limit"));
}
#[test]
fn json_final_newline_cannot_cross_the_bound() {
    let mut out = Bounded::new(3);
    serde_json::to_writer_pretty(&mut out, &"x").unwrap();
    assert!(out.write_all(b"\n").is_err());
    assert_eq!(out.cursor.into_inner(), b"\"x\"");
}
#[test]
fn zip_combined_members_cannot_exceed_64_mib() {
    let data = vec![b'x'; 33_554_432];
    let err = match zip(&data, &data) {
        Err(e) => e,
        Ok(_) => panic!("ZIP member total must be bounded"),
    };
    assert!(err.contains("output limit"));
}
