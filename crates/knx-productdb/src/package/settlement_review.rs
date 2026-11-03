//! Independent raw ZIP fixtures exercise physical identity, codecs and bounded inventory.
use super::*;
#[derive(Clone)]
struct Entry {
    name: Vec<u8>,
    data: Vec<u8>,
    extra: Vec<u8>,
    flags: u16,
    method: u16,
    mode: u32,
}
fn entry(name: &[u8], data: &[u8]) -> Entry {
    Entry {
        name: name.to_vec(),
        data: data.to_vec(),
        extra: vec![],
        flags: 0,
        method: 0,
        mode: if name.ends_with(b"/") {
            0o040755
        } else {
            0o100644
        },
    }
}
fn u16b(v: &mut Vec<u8>, n: u16) {
    v.extend(n.to_le_bytes());
}
fn u32b(v: &mut Vec<u8>, n: u32) {
    v.extend(n.to_le_bytes());
}
fn raw(entries: &[Entry], order: &[usize]) -> Vec<u8> {
    let mut v = vec![];
    let mut offsets = vec![0; entries.len()];
    for &i in order {
        let e = &entries[i];
        offsets[i] = v.len() as u32;
        v.extend(b"PK\x03\x04");
        u16b(&mut v, 20);
        u16b(&mut v, e.flags);
        u16b(&mut v, e.method);
        u32b(&mut v, 0);
        u32b(&mut v, crc32fast::hash(&e.data));
        u32b(&mut v, e.data.len() as u32);
        u32b(&mut v, e.data.len() as u32);
        u16b(&mut v, e.name.len() as u16);
        u16b(&mut v, e.extra.len() as u16);
        v.extend(&e.name);
        v.extend(&e.extra);
        v.extend(&e.data);
    }
    let start = v.len() as u32;
    for (i, e) in entries.iter().enumerate() {
        v.extend(b"PK\x01\x02");
        u16b(&mut v, 0x314);
        u16b(&mut v, 20);
        u16b(&mut v, e.flags);
        u16b(&mut v, e.method);
        u32b(&mut v, 0);
        u32b(&mut v, crc32fast::hash(&e.data));
        u32b(&mut v, e.data.len() as u32);
        u32b(&mut v, e.data.len() as u32);
        u16b(&mut v, e.name.len() as u16);
        u16b(&mut v, e.extra.len() as u16);
        u16b(&mut v, 0);
        u16b(&mut v, 0);
        u16b(&mut v, 0);
        u32b(&mut v, e.mode << 16);
        u32b(&mut v, offsets[i]);
        v.extend(&e.name);
        v.extend(&e.extra);
    }
    let size = v.len() as u32 - start;
    v.extend(b"PK\x05\x06");
    u16b(&mut v, 0);
    u16b(&mut v, 0);
    u16b(&mut v, entries.len() as u16);
    u16b(&mut v, entries.len() as u16);
    u32b(&mut v, size);
    u32b(&mut v, start);
    u16b(&mut v, 0);
    v
}
fn limits() -> PackageLimits {
    PackageLimits {
        max_member_size: 64,
        max_expanded_size: 128,
    }
}
fn inv(bytes: &[u8]) -> Result<Vec<SettlementMember>, PackageError> {
    settlement_inventory(bytes, limits())
}
fn cd(bytes: &[u8]) -> usize {
    let e = bytes.len() - 22;
    u32::from_le_bytes(bytes[e + 16..e + 20].try_into().unwrap()) as usize
}
#[test]
fn review_inventory_keeps_unknown_opaque_and_empty_directory() {
    let entries = [entry(b"empty/", b""), entry(b"plugin.bin", b"opaque\0\xff")];
    let members = inv(&raw(&entries, &[0, 1])).unwrap();
    assert_eq!(members.len(), 2);
    assert!(members[0].directory);
    assert_eq!(members[1].bytes, b"opaque\0\xff");
}
#[test]
fn review_local_order_changes_even_with_identical_central_order() {
    let entries = [entry(b"a", b"one"), entry(b"b", b"two")];
    let a = inv(&raw(&entries, &[0, 1])).unwrap();
    let b = inv(&raw(&entries, &[1, 0])).unwrap();
    assert_eq!(
        a.iter().map(|m| &m.name).collect::<Vec<_>>(),
        b.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    assert_ne!(a, b);
    assert_eq!(b[0].local_ordinal, 1);
    assert_eq!(b[1].local_ordinal, 0);
}
#[test]
fn review_central_order_is_preserved() {
    let a = inv(&raw(&[entry(b"a", b"one"), entry(b"b", b"two")], &[0, 1])).unwrap();
    let b = inv(&raw(&[entry(b"b", b"two"), entry(b"a", b"one")], &[1, 0])).unwrap();
    assert_ne!(a, b);
    assert_eq!(b[0].name, "b");
}
#[test]
fn review_aggregate_budget_not_only_per_member() {
    let bytes = raw(
        &[entry(b"a", b"12345678"), entry(b"b", b"12345678")],
        &[0, 1],
    );
    assert_eq!(inv(&bytes).unwrap().len(), 2);
    assert!(settlement_inventory(
        &bytes,
        PackageLimits {
            max_member_size: 16,
            max_expanded_size: 15
        }
    )
    .is_err());
}
#[test]
fn review_local_central_name_flags_method_crc_size_must_agree() {
    let good = raw(&[entry(b"a", b"abc")], &[0]);
    assert_eq!(inv(&good).unwrap().len(), 1);
    for at in [30, 6, 8, 14, 18, 22] {
        let mut bad = good.clone();
        bad[at] ^= 1;
        assert!(inv(&bad).is_err(), "offset {at}");
    }
}
#[test]
fn review_crc_failure_and_unsupported_codec_are_not_skipped() {
    let mut bad = raw(&[entry(b"opaque.bin", b"abc")], &[0]);
    let at = 30 + b"opaque.bin".len();
    bad[at] ^= 1;
    assert!(inv(&bad).is_err());
    let mut e = entry(b"opaque.bin", b"abc");
    e.method = 99;
    assert!(inv(&raw(&[e], &[0])).is_err());
}
#[test]
fn review_empty_encrypted_directories_and_special_modes_refuse() {
    for flags in [1, 64] {
        let mut e = entry(b"empty/", b"");
        e.flags = flags;
        assert!(inv(&raw(&[e], &[0])).is_err());
    }
    for mode in [0o120777, 0o020600, 0o010600, 0o060600, 0o040755] {
        let mut e = entry(b"file", b"");
        e.mode = mode;
        assert!(inv(&raw(&[e], &[0])).is_err());
    }
    let mut e = entry(b"dir/", b"");
    e.mode = 0o100644;
    assert!(inv(&raw(&[e], &[0])).is_err());
}
#[test]
fn review_raw_utf8_decoding_and_duplicate_identities() {
    let mut e = entry(&[0xff], b"");
    e.flags = 0x800;
    assert!(inv(&raw(&[e], &[0])).is_err());
    let e = entry(&[0x82], b"");
    let x = inv(&raw(&[e], &[0])).unwrap();
    assert_eq!(x[0].raw_name, [0x82]);
    assert_eq!(x[0].name, "é");
    assert!(inv(&raw(&[entry(b"a", b"x"), entry(b"a", b"y")], &[0, 1])).is_err());
    assert!(inv(&raw(&[entry(b"a", b"x"), entry(b"a/child", b"y")], &[0, 1])).is_err());
}
fn unicode(raw_name: &[u8], value: &str) -> Vec<u8> {
    let mut x = vec![];
    u16b(&mut x, 0x7075);
    u16b(&mut x, (5 + value.len()) as u16);
    x.push(1);
    u32b(&mut x, crc32fast::hash(raw_name));
    x.extend(value.as_bytes());
    x
}
#[test]
fn review_unicode_path_alias_and_local_central_override_disagreement_refuse() {
    let mut a = entry(b"a", b"x");
    a.extra = unicode(b"a", "same");
    let mut b = entry(b"b", b"y");
    b.extra = unicode(b"b", "same");
    assert!(inv(&raw(&[a.clone(), b], &[0, 1])).is_err());
    let mut good = raw(&[a], &[0]);
    assert_eq!(inv(&good).unwrap()[0].name, "same");
    let c = cd(&good);
    let last = c + 46 + 1 + 4 + 5;
    good[last] ^= 1;
    assert!(inv(&good).is_err());
}
#[test]
fn review_unknown_extra_fields_are_preserved_not_repaired() {
    let mut a = entry(b"a", b"x");
    a.extra = vec![0xfe, 0xca, 1, 0, 7];
    let mut b = a.clone();
    b.extra[4] = 8;
    let old = inv(&raw(&[a], &[0])).unwrap();
    let new = inv(&raw(&[b], &[0])).unwrap();
    assert_ne!(old, new);
    assert_eq!(old[0].local_extra, [0xfe, 0xca, 1, 0, 7]);
    assert_eq!(old[0].central_extra, old[0].local_extra);
}
#[test]
fn review_unindexed_local_record_is_refused_only_by_strict_inventory() {
    let entries = [entry(b"a", b"x")];
    let good = raw(&entries, &[0]);
    let mut bad = good.clone();
    let c = cd(&bad);
    let orphan = raw(&[entry(b"hidden-plugin.bin", b"opaque")], &[0]);
    let local = &orphan[..cd(&orphan)];
    bad.splice(c..c, local.iter().copied());
    let e = bad.len() - 22;
    bad[e + 16..e + 20].copy_from_slice(&((c + local.len()) as u32).to_le_bytes());
    // Ordinary central-directory admission is unchanged; only the new comparison
    // promises physical exhaustiveness and must refuse the unindexed region.
    open_checked_archive(&bad, limits(), Cancellation::Quiet).unwrap();
    assert_eq!(inv(&good).unwrap().len(), 1);
    assert!(inv(&bad).is_err());
}
fn deflated(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "a",
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    w.write_all(data).unwrap();
    w.finish().unwrap().into_inner()
}
#[test]
fn review_deflate_honest_and_underdeclared_expansion() {
    let good = deflated(&[b'x'; 48]);
    assert_eq!(inv(&good).unwrap()[0].bytes, [b'x'; 48]);
    let mut bad = good;
    let c = cd(&bad);
    bad[22..26].copy_from_slice(&8u32.to_le_bytes());
    bad[c + 24..c + 28].copy_from_slice(&8u32.to_le_bytes());
    assert!(settlement_inventory(
        &bad,
        PackageLimits {
            max_member_size: 16,
            max_expanded_size: 16
        }
    )
    .is_err());
}
#[test]
fn review_trailing_compressed_bytes_do_not_disappear() {
    let mut bad = deflated(b"abc");
    let c = cd(&bad);
    let n = u32::from_le_bytes(bad[18..22].try_into().unwrap());
    bad.splice(c..c, [0u8]);
    let shifted = c + 1;
    bad[18..22].copy_from_slice(&(n + 1).to_le_bytes());
    bad[shifted + 20..shifted + 24].copy_from_slice(&(n + 1).to_le_bytes());
    let e = bad.len() - 22;
    bad[e + 16..e + 20].copy_from_slice(&(shifted as u32).to_le_bytes());
    assert!(inv(&bad).is_err());
}
#[test]
fn review_malformed_extra_field_extent_refuses() {
    let mut a = entry(b"a", b"x");
    a.extra = vec![0xfe, 0xca, 2, 0, 7];
    assert!(inv(&raw(&[a], &[0])).is_err());
}

fn inserted_physical_region(good: &[u8], at: usize, region: &[u8]) -> Vec<u8> {
    let start = cd(good);
    assert!(at <= start);
    let mut changed = good.to_vec();
    changed.splice(at..at, region.iter().copied());
    let mut central = start + region.len();
    let eocd = changed.len() - 22;
    assert_eq!(&changed[eocd..eocd + 4], b"PK\x05\x06");
    changed[eocd + 16..eocd + 20].copy_from_slice(&u32::try_from(central).unwrap().to_le_bytes());
    while central < eocd {
        assert_eq!(&changed[central..central + 4], b"PK\x01\x02");
        let old = u32::from_le_bytes(changed[central + 42..central + 46].try_into().unwrap());
        if usize::try_from(old).unwrap() >= at {
            let shifted = old + u32::try_from(region.len()).unwrap();
            changed[central + 42..central + 46].copy_from_slice(&shifted.to_le_bytes());
        }
        let mut record_len = 46;
        for field in [28, 30, 32] {
            record_len += usize::from(u16::from_le_bytes(
                changed[central + field..central + field + 2]
                    .try_into()
                    .unwrap(),
            ));
        }
        central += record_len;
    }
    assert_eq!(central, eocd);
    changed
}

#[test]
fn review_unaccounted_prefix_interrecord_and_predirectory_regions_refuse() {
    let entries = [entry(b"a", b"one"), entry(b"b", b"two")];
    let good = raw(&entries, &[0, 1]);
    let start = cd(&good);
    let second_header = start + 46 + entries[0].name.len();
    let second = usize::try_from(u32::from_le_bytes(
        good[second_header + 42..second_header + 46]
            .try_into()
            .unwrap(),
    ))
    .unwrap();
    let orphan = raw(&[entry(b"hidden.bin", b"opaque")], &[0]);
    for at in [0, second, start] {
        for region in [b"unexplained padding".as_slice(), &orphan[..cd(&orphan)]] {
            let changed = inserted_physical_region(&good, at, region);
            open_checked_archive(&changed, limits(), Cancellation::Quiet).unwrap();
            assert!(inv(&changed).is_err(), "unaccounted region at {at}");
        }
    }
}

#[test]
fn review_indexed_payload_and_declared_comment_signatures_are_not_orphan_records() {
    let good = raw(&[entry(b"opaque.bin", b"beforePK\x03\x04after")], &[0]);
    let mut comment = good.clone();
    let eocd = comment.len() - 22;
    let text = b"declared comment: PK\x03\x04 not a file";
    comment[eocd + 20..eocd + 22]
        .copy_from_slice(&u16::try_from(text.len()).unwrap().to_le_bytes());
    comment.extend(text);
    assert_eq!(inv(&good).unwrap(), inv(&comment).unwrap());
}

#[test]
fn review_complete_local_ranges_include_signed_and_unsigned_data_descriptors() {
    let data = b"abc";
    let good = raw(&[entry(b"a", data)], &[0]);
    let start = cd(&good);
    for signed in [false, true] {
        let mut descriptor = Vec::new();
        if signed {
            descriptor.extend(b"PK\x07\x08");
        }
        u32b(&mut descriptor, crc32fast::hash(data));
        u32b(&mut descriptor, u32::try_from(data.len()).unwrap());
        u32b(&mut descriptor, u32::try_from(data.len()).unwrap());
        let mut changed = inserted_physical_region(&good, start, &descriptor);
        let central = start + descriptor.len();
        changed[6] |= 8;
        changed[14..26].fill(0);
        changed[central + 8] |= 8;
        let members = inv(&changed).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].bytes, data);
    }
}
