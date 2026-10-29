//! Pins independent property-description vectors and lossless wrong-width handling.

use super::*;

fn frame(apci: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![
        0x29,
        0,
        0xbc,
        0x60,
        0x11,
        0x43,
        0x11,
        0xfe,
        (payload.len() + 1) as u8,
        0x43,
        apci,
    ];
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn description_vectors_decode_and_encode_exactly() {
    let request = frame(0xd8, &[0x23, 0x16, 0x37]);
    let decoded = decode_l_data(&request).unwrap();
    assert_eq!(
        decoded.service,
        ApplicationService::PropertyDescriptionRead {
            object_index: 0x23,
            property_id: 0x16,
            property_index: 0x37,
        }
    );
    assert_eq!(encode_l_data(&decoded).unwrap(), request);
    let response = frame(0xd9, &[0x23, 0x16, 0x37, 0xc2, 0xa5, 0x67, 0x89]);
    let decoded = decode_l_data(&response).unwrap();
    assert_eq!(
        decoded.service,
        ApplicationService::PropertyDescriptionResponse {
            object_index: 0x23,
            property_id: 0x16,
            property_index: 0x37,
            description: PropertyDescription::from_raw([0xc2, 0xa5, 0x67, 0x89]),
        }
    );
    assert_eq!(encode_l_data(&decoded).unwrap(), response);
}

#[test]
fn description_wrong_widths_stay_raw_and_roundtrip() {
    for (apci, width) in [(0xd8, 3), (0xd9, 7)] {
        for len in 0..=10 {
            if len == width {
                continue;
            }
            let payload = vec![0x39; len];
            let bytes = frame(apci, &payload);
            let decoded = decode_l_data(&bytes).unwrap();
            assert_eq!(
                decoded.service,
                ApplicationService::Other {
                    apci: 0x300 | u16::from(apci),
                    data: payload,
                }
            );
            assert_eq!(encode_l_data(&decoded).unwrap(), bytes);
        }
    }
}

#[test]
fn description_raw_getters_preserve_every_bit() {
    for flags in 0..=255 {
        let d = PropertyDescription::from_raw([flags, 0xa5, 0x67, 0x89]);
        assert_eq!(d.raw(), [flags, 0xa5, 0x67, 0x89]);
        assert_eq!(d.write_enabled(), flags & 0x80 != 0);
        assert_eq!(d.legacy_bit(), flags & 0x40 != 0);
        assert_eq!(d.pdt(), flags & 0x3f);
        assert_eq!(d.upper_max_elements(), 10);
        assert_eq!(d.max_elements(), 0x567);
        assert_eq!((d.read_access(), d.write_access()), (8, 9));
    }
}
