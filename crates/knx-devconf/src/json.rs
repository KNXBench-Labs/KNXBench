//! Parses strict JSON: refuses duplicate members, fractions, exponents and oversized integers.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::collections::HashSet;
use std::fmt;

/// A JSON value with integers only and object members in source order.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Short name of the value's kind for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "boolean",
            Json::Int(_) => "integer",
            Json::Str(_) => "string",
            Json::Array(_) => "array",
            Json::Object(_) => "object",
        }
    }

    /// Equality where object member order does not matter.
    pub fn same(&self, other: &Json) -> bool {
        match (self, other) {
            (Json::Object(a), Json::Object(b)) => {
                a.len() == b.len()
                    && a.iter().all(|(name, value)| {
                        b.iter().any(|(other_name, other_value)| {
                            name == other_name && value.same(other_value)
                        })
                    })
            }
            (Json::Array(a), Json::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same(y))
            }
            _ => self == other,
        }
    }
}

/// Parses `bytes` as one strict JSON value of at most `limit` bytes.
pub fn parse(bytes: &[u8], limit: usize) -> Result<Json, String> {
    if bytes.len() > limit {
        return Err(format!(
            "input is {} bytes, the limit is {limit}",
            bytes.len()
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Strict
        .deserialize(&mut deserializer)
        .map_err(|e| e.to_string())?;
    deserializer.end().map_err(|e| e.to_string())?;
    Ok(value)
}

struct Strict;

impl<'de> DeserializeSeed<'de> for Strict {
    type Value = Json;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Json, E> {
        Ok(Json::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Json, E> {
        Ok(Json::Int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Json, E> {
        i64::try_from(v)
            .map(Json::Int)
            .map_err(|_| E::custom("integer exceeds the signed 64-bit range"))
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Json, E> {
        Err(E::custom(
            "numbers must be integers without fraction or exponent",
        ))
    }

    fn visit_str<E>(self, v: &str) -> Result<Json, E> {
        Ok(Json::Str(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Json, E> {
        Ok(Json::Str(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(Strict)? {
            items.push(item);
        }
        Ok(Json::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut members = Vec::new();
        let mut seen = HashSet::new();
        while let Some(name) = map.next_key::<String>()? {
            if !seen.insert(name.clone()) {
                return Err(de::Error::custom(format!("duplicate member \"{name}\"")));
            }
            let value = map.next_value_seed(Strict)?;
            members.push((name, value));
        }
        Ok(Json::Object(members))
    }
}

/// An object whose member set has been checked against an exact list.
#[derive(Debug)]
pub(crate) struct Fields<'a> {
    pub path: String,
    members: &'a [(String, Json)],
}

/// Checks that `json` is an object with all `required` and only `required`/`optional` members.
pub(crate) fn fields<'a>(
    json: &'a Json,
    path: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<Fields<'a>, String> {
    let Json::Object(members) = json else {
        return Err(format!("{path}: expected an object, found {}", json.kind()));
    };
    for (name, _) in members {
        if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
            return Err(format!("{path}: unknown member \"{name}\""));
        }
    }
    for name in required {
        if !members.iter().any(|(m, _)| m == name) {
            return Err(format!("{path}: missing member \"{name}\""));
        }
    }
    Ok(Fields {
        path: path.to_owned(),
        members,
    })
}

impl<'a> Fields<'a> {
    pub fn opt(&self, name: &str) -> Option<&'a Json> {
        self.members.iter().find(|(m, _)| m == name).map(|(_, v)| v)
    }

    pub fn req(&self, name: &str) -> &'a Json {
        self.opt(name).expect("required member was checked")
    }

    pub fn at(&self, name: &str) -> String {
        format!("{}/{name}", self.path)
    }

    pub fn int(&self, name: &str, min: i64, max: i64) -> Result<i64, String> {
        match self.req(name) {
            Json::Int(v) if (min..=max).contains(v) => Ok(*v),
            Json::Int(v) => Err(format!("{}: {v} is outside {min}..{max}", self.at(name))),
            other => Err(format!(
                "{}: expected an integer, found {}",
                self.at(name),
                other.kind()
            )),
        }
    }

    pub fn count(&self, name: &str, max: usize) -> Result<usize, String> {
        let limit = i64::try_from(max).unwrap_or(i64::MAX);
        self.int(name, 0, limit)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX))
    }

    pub fn str(&self, name: &str) -> Result<&'a str, String> {
        str_of(self.req(name), &self.at(name))
    }

    pub fn opt_str(&self, name: &str) -> Result<Option<&'a str>, String> {
        self.opt(name)
            .map(|v| str_of(v, &self.at(name)))
            .transpose()
    }
}

pub(crate) fn str_of<'a>(json: &'a Json, path: &str) -> Result<&'a str, String> {
    match json {
        Json::Str(s) => Ok(s),
        other => Err(format!("{path}: expected a string, found {}", other.kind())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_members_are_refused_not_last_wins() {
        let err = parse(br#"{"a":1,"a":2}"#, 100).unwrap_err();
        assert!(err.contains("duplicate member \"a\""), "{err}");
    }

    #[test]
    fn fractions_exponents_and_huge_integers_are_refused() {
        for text in [
            "1.0",
            "1e2",
            "-0",
            "18446744073709551616",
            "9223372036854775808",
        ] {
            assert!(parse(text.as_bytes(), 100).is_err(), "{text} accepted");
        }
        assert_eq!(parse(b"-9223372036854775808", 100), Ok(Json::Int(i64::MIN)));
    }

    #[test]
    fn size_limit_and_trailing_data_are_refused() {
        assert!(parse(b"[1,2,3]", 6).unwrap_err().contains("limit is 6"));
        assert!(parse(b"1 2", 10).is_err());
        assert!(
            parse(br#""\ud800""#, 10).is_err(),
            "lone surrogate accepted"
        );
    }

    #[test]
    fn object_equality_ignores_member_order() {
        let a = parse(br#"{"x":1,"y":[1,{"z":2,"w":3}]}"#, 100).unwrap();
        let b = parse(br#"{"y":[1,{"w":3,"z":2}],"x":1}"#, 100).unwrap();
        let c = parse(br#"{"y":[{"w":3,"z":2},1],"x":1}"#, 100).unwrap();
        assert!(a.same(&b));
        assert!(!a.same(&c), "array order must matter");
    }

    #[test]
    fn exact_member_sets_name_the_problem() {
        let j = parse(br#"{"a":1,"b":2}"#, 100).unwrap();
        assert!(fields(&j, "/x", &["a"], &[])
            .unwrap_err()
            .contains("unknown member \"b\""));
        assert!(fields(&j, "/x", &["a", "b", "c"], &[])
            .unwrap_err()
            .contains("missing member \"c\""));
        let f = fields(&j, "/x", &["a"], &["b"]).unwrap();
        assert_eq!(f.int("a", 0, 1), Ok(1));
        assert!(f.int("b", 0, 1).unwrap_err().contains("outside 0..1"));
    }
}
