//! Direct admission-boundary controls; constants pinned independently.
use super::*;

#[test]
fn entity_cap_admits_100000_and_refuses_next_record() {
    let mut b = Budget::default();
    b.entities(100_000).unwrap();
    assert_eq!(b.entities, 100_000);
    assert!(b.entities(1).is_err());
}
#[test]
fn input_cap_admits_32_mib_and_refuses_next_byte() {
    let mut b = Budget::default();
    b.bytes(33_554_432).unwrap();
    assert_eq!(b.bytes, 33_554_432);
    assert!(b.bytes(1).is_err());
}
#[test]
fn references_cap_admits_one_million_and_refuses_next_entry() {
    let mut b = Budget::default();
    b.refs(1_000_000).unwrap();
    assert_eq!(b.references, 1_000_000);
    assert!(b.refs(1).is_err());
}
