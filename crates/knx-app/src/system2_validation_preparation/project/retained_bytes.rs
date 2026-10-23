//! Canonical compact retention for project-v2 evidence, not shared plan/recovery octets.
use super::LIMIT;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(super) mod bytes {
    use super::*;
    use base64::{display::Base64Display, engine::general_purpose::STANDARD, Engine};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        if v.len() > LIMIT || base64::encoded_len(v.len(), true).is_none_or(|n| n > LIMIT) {
            return Err(serde::ser::Error::custom("retained byte limit"));
        }
        s.collect_str(&Base64Display::new(v, &STANDARD))
    }
    pub(super) fn decode_bounded(
        encoded: &str,
        encoded_limit: usize,
        decoded_limit: usize,
    ) -> Result<Vec<u8>, &'static str> {
        if encoded.len() > encoded_limit {
            return Err("encoded byte limit");
        }
        if !encoded.len().is_multiple_of(4) {
            return Err("invalid canonical base64");
        }
        let padding = encoded
            .as_bytes()
            .iter()
            .rev()
            .take(2)
            .take_while(|&&b| b == b'=')
            .count();
        let decoded = (encoded.len() / 4)
            .checked_mul(3)
            .and_then(|n| n.checked_sub(padding))
            .ok_or("invalid canonical base64")?;
        if decoded > decoded_limit {
            return Err("decoded byte limit");
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(decoded)
            .map_err(|_| "retained byte allocation")?;
        output.resize(decoded, 0);
        let written = STANDARD
            .decode_slice(encoded, &mut output)
            .map_err(|_| "invalid canonical base64")?;
        if written != decoded {
            return Err("invalid canonical base64");
        }
        Ok(output)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        struct Bytes;
        impl<'de> serde::de::Visitor<'de> for Bytes {
            type Value = Vec<u8>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bounded canonical padded standard base64 byte string")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Vec<u8>, E> {
                decode_bounded(v, LIMIT, LIMIT).map_err(E::custom)
            }
        }
        d.deserialize_str(Bytes)
    }
}

fn member_total_ok(a: usize, b: usize, limit: usize) -> bool {
    a.checked_add(b).is_some_and(|n| n <= limit)
}

pub(super) mod members {
    use super::*;
    pub fn serialize<S: Serializer>(v: &[Vec<u8>; 2], s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Bytes<'a>(#[serde(with = "bytes")] &'a [u8]);
        if !member_total_ok(v[0].len(), v[1].len(), LIMIT) {
            return Err(serde::ser::Error::custom("retained source aggregate limit"));
        }
        [Bytes(&v[0]), Bytes(&v[1])].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[Vec<u8>; 2], D::Error> {
        #[derive(Deserialize)]
        struct Bytes(#[serde(with = "bytes")] Vec<u8>);
        let [a, b] = <[Bytes; 2]>::deserialize(d)?;
        if !member_total_ok(a.0.len(), b.0.len(), LIMIT) {
            return Err(serde::de::Error::custom("retained source aggregate limit"));
        }
        Ok([a.0, b.0])
    }
}

#[cfg(test)]
mod tests {
    use super::super::LIMIT;
    use super::*;
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Blob {
        #[serde(with = "bytes")]
        bytes: Vec<u8>,
    }
    #[test]
    fn encoded_limit_is_inclusive_and_checked_before_decode() {
        assert_eq!(bytes::decode_bounded("AQID", 4, 3).unwrap(), vec![1, 2, 3]);
        assert_eq!(
            bytes::decode_bounded("AQID", 3, 3),
            Err("encoded byte limit")
        );
        assert_eq!(
            bytes::decode_bounded("!!!!", 3, 3),
            Err("encoded byte limit")
        );
    }
    #[test]
    fn decoded_limit_uses_padding_exactly() {
        for (text, n) in [("AA==", 1), ("AAE=", 2), ("AAEC", 3)] {
            assert_eq!(bytes::decode_bounded(text, 4, n).unwrap().len(), n);
            assert_eq!(
                bytes::decode_bounded(text, 4, n - 1),
                Err("decoded byte limit")
            );
        }
    }
    #[test]
    fn ordered_exact_two_string_members_roundtrip() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Pair {
            #[serde(with = "members")]
            member_bytes: [Vec<u8>; 2],
        }
        let pair = Pair {
            member_bytes: [vec![0], vec![255]],
        };
        let json = serde_json::to_string(&pair).unwrap();
        assert_eq!(json, r#"{"member_bytes":["AA==","/w=="]}"#);
        assert_eq!(serde_json::from_str::<Pair>(&json).unwrap(), pair);
        for value in [
            serde_json::json!([]),
            serde_json::json!([""]),
            serde_json::json!(["", "", ""]),
            serde_json::json!([[], []]),
            serde_json::json!(["AA==", "AB=="]),
        ] {
            assert!(
                serde_json::from_value::<Pair>(serde_json::json!({"member_bytes": value})).is_err()
            );
        }
    }
    #[test]
    fn production_member_aggregate_is_enforced_on_deserialization() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Pair(#[serde(with = "members")] [Vec<u8>; 2]);
        // Two individually legal strings, decoded sum just beyond LIMIT. This
        // deliberately bypasses bounded_read to isolate the pair visitor guard.
        let decoded_each = (LIMIT / 2 / 3 + 1) * 3;
        assert!(decoded_each < LIMIT && decoded_each * 2 > LIMIT);
        let encoded = "A".repeat(decoded_each / 3 * 4);
        assert!(encoded.len() < LIMIT);
        let input = format!("[\"{encoded}\",\"{encoded}\"]");
        let error = match serde_json::from_str::<Pair>(&input) {
            Err(error) => error,
            Ok(_) => panic!("decoded member aggregate must be refused"),
        };
        assert!(error
            .to_string()
            .contains("retained source aggregate limit"));
    }
    #[test]
    fn production_member_aggregate_is_enforced_on_serialization() {
        #[derive(Serialize)]
        struct Pair(#[serde(with = "members")] [Vec<u8>; 2]);
        let decoded_each = (LIMIT / 2 / 3 + 1) * 3;
        let value = Pair([vec![0; decoded_each], vec![0; decoded_each]]);
        let error = match serde_json::to_vec(&value) {
            Err(error) => error,
            Ok(_) => panic!("serialized member aggregate must be refused"),
        };
        assert!(error
            .to_string()
            .contains("retained source aggregate limit"));
    }
    #[test]
    fn canonical_string_roundtrip() {
        let blob = Blob {
            bytes: vec![0, 1, 254, 255],
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert_eq!(json, r#"{"bytes":"AAH+/w=="}"#);
        assert_eq!(serde_json::from_str::<Blob>(&json).unwrap(), blob);
    }
    #[test]
    fn rejects_non_string_json_without_numeric_fallback() {
        for value in [
            serde_json::json!([]),
            serde_json::json!([0, 1]),
            serde_json::Value::Null,
            serde_json::json!(1),
            serde_json::json!({"base64":""}),
        ] {
            assert!(serde_json::from_value::<Blob>(serde_json::json!({"bytes":value})).is_err());
        }
    }
    #[test]
    fn rejects_malformed_padding_alphabet_whitespace_and_trailing_bits() {
        for text in [
            "A", "AA", "AAA", "A===", "====", "AA===", "AA=", "AA=A", "=AAA", "AA==AA==", "AA==\n",
            " AAAAAA=", "AA\r=", "AA\t=", "-w==", "_w==", "!!!!", "éAA", "AB==", "AAB=",
        ] {
            assert!(
                bytes::decode_bounded(text, 128, 128).is_err(),
                "accepted {text:?}"
            );
        }
    }
    #[test]
    fn empty_and_every_padding_class_and_value_roundtrip() {
        for n in 0..1024 {
            let raw: Vec<u8> = (0..n).map(|i| (i * 73 + 19) as u8).collect();
            let blob = Blob { bytes: raw };
            let encoded = serde_json::to_vec_pretty(&blob).unwrap();
            assert_eq!(serde_json::from_slice::<Blob>(&encoded).unwrap(), blob);
            assert_eq!(
                serde_json::from_value::<Blob>(serde_json::to_value(&blob).unwrap()).unwrap(),
                blob
            );
        }
    }
    #[test]
    fn actual_16mib_encoded_boundary_and_serializer_refusal() {
        // Production encoded and decoded limits both remain 16 MiB. Field ceiling
        // alone is insufficient for an envelope: enclosing bytes count separately.
        let text = "A".repeat(LIMIT);
        assert_eq!(
            bytes::decode_bounded(&text, LIMIT, LIMIT).unwrap().len(),
            LIMIT / 4 * 3
        );
        let mut over = text;
        over.push_str("AAAA");
        assert!(matches!(
            bytes::decode_bounded(&over, LIMIT, LIMIT),
            Err("encoded byte limit")
        ));
    }
    #[test]
    fn json_escaping_is_accepted_but_decoded_string_whitespace_is_not() {
        let blob: Blob = serde_json::from_str(r#"{"bytes":"\u0041AH+\/w=="}"#).unwrap();
        assert_eq!(blob.bytes, [0, 1, 254, 255]);
        assert!(serde_json::from_str::<Blob>(r#"{"bytes":"AA==\n"}"#).is_err());
    }

    #[test]
    fn streaming_serializer_honors_counter_refusal_inside_bytes_and_at_final_quote() {
        use std::io::{self, Write};
        #[derive(Serialize)]
        struct Scalar<'a>(#[serde(with = "bytes")] &'a [u8]);
        struct Counter {
            used: usize,
            limit: usize,
            largest_write: usize,
        }
        impl Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.largest_write = self.largest_write.max(bytes.len());
                let next = self
                    .used
                    .checked_add(bytes.len())
                    .filter(|&n| n <= self.limit)
                    .ok_or_else(|| io::Error::other("counter refusal"))?;
                self.used = next;
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let raw = vec![0; 4096];
        let exact = 5466; // 4 * ceil(4096 / 3), plus the two JSON quotes.
        for pretty in [false, true] {
            for limit in [exact, exact - 1, 17] {
                let mut counter = Counter {
                    used: 0,
                    limit,
                    largest_write: 0,
                };
                let result = if pretty {
                    serde_json::to_writer_pretty(&mut counter, &Scalar(&raw))
                } else {
                    serde_json::to_writer(&mut counter, &Scalar(&raw))
                };
                if limit == exact {
                    result.unwrap();
                    assert_eq!(counter.used, exact);
                } else {
                    assert!(result.unwrap_err().to_string().contains("counter refusal"));
                    assert!(counter.used <= limit);
                }
                // collect_str forwards Base64Display chunks, not a full encoded String.
                assert!(counter.largest_write <= 1024);
            }
        }
    }

    #[test]
    fn member_aggregate_is_inclusive_and_overflow_safe() {
        assert!(member_total_ok(2, 2, 4));
        assert!(!member_total_ok(2, 2, 3));
        assert!(!member_total_ok(usize::MAX, 1, usize::MAX));
    }

    #[test]
    fn scalar_field_ceiling_is_inclusive_but_does_not_admit_an_outer_document() {
        assert_eq!(LIMIT, 16_777_216);
        let blob = Blob {
            bytes: vec![0; LIMIT / 4 * 3],
        };
        let encoded = serde_json::to_vec(&blob).unwrap();
        assert_eq!(encoded.len(), LIMIT + r#"{"bytes":""}"#.len());
        assert_eq!(
            serde_json::from_slice::<Blob>(&encoded)
                .unwrap()
                .bytes
                .len(),
            blob.bytes.len()
        );
        let input = format!(r#"{{"bytes":"{}"}}"#, "A".repeat(LIMIT + 4));
        let error = match serde_json::from_str::<Blob>(&input) {
            Err(error) => error,
            Ok(_) => panic!("serde scalar must enforce encoded limit"),
        };
        assert!(error.to_string().contains("encoded byte limit"));
    }

    #[test]
    fn serializer_refuses_encoded_or_decoded_field_limit() {
        for size in [LIMIT / 4 * 3 + 1, LIMIT + 1] {
            let blob = Blob {
                bytes: vec![0; size],
            };
            let error = match serde_json::to_vec(&blob) {
                Err(error) => error,
                Ok(_) => panic!("serialized scalar byte limit must be refused"),
            };
            assert!(error.to_string().contains("retained byte limit"));
        }
    }
}
