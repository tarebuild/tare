//! Exact JSON integers: ordinary numbers through 2^53-1, decimal strings above it.
use serde::{de::{self, Visitor}, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

pub const MAX_SAFE_INTEGER: u64 = (1u64 << 53) - 1;

pub fn value(number: u64) -> Value {
    if number <= MAX_SAFE_INTEGER { Value::from(number) } else { Value::String(number.to_string()) }
}
pub fn integer(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| decimal(value.as_str()?))
}
fn decimal(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())).then(|| text.parse().ok()).flatten()
}
pub fn serialize<S: Serializer>(number: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    if *number <= MAX_SAFE_INTEGER { serializer.serialize_u64(*number) }
    else { serializer.serialize_str(&number.to_string()) }
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    struct Exact;
    impl<'de> Visitor<'de> for Exact {
        type Value = u64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("an unsigned integer or exact decimal string") }
        fn visit_u64<E: de::Error>(self, value: u64) -> Result<u64,E> { Ok(value) }
        fn visit_u32<E: de::Error>(self, value: u32) -> Result<u64,E> { Ok(value.into()) }
        fn visit_i64<E: de::Error>(self, value: i64) -> Result<u64,E> { u64::try_from(value).map_err(E::custom) }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<u64,E> { decimal(value).ok_or_else(|| E::custom("invalid or overflowing unsigned decimal integer")) }
    }
    deserializer.deserialize_any(Exact)
}

pub mod option {
    use super::*;
    pub fn serialize<S: Serializer>(number: &Option<u64>, serializer: S) -> Result<S::Ok,S::Error> {
        number.map(value).serialize(serializer)
    }
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<Option<u64>,D::Error>{
        Option::<Value>::deserialize(deserializer)?.map(|value|integer(&value).ok_or_else(||de::Error::custom("invalid unsigned integer"))).transpose()
    }
}
pub mod signed_option {
    use super::*;
    pub fn serialize<S:Serializer>(number:&Option<i64>,serializer:S)->Result<S::Ok,S::Error>{
        number.map(|number|if number.unsigned_abs()<=MAX_SAFE_INTEGER{Value::from(number)}else{Value::String(number.to_string())}).serialize(serializer)
    }
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<Option<i64>,D::Error>{
        Option::<Value>::deserialize(deserializer)?.map(|value|value.as_i64().or_else(||value.as_str()?.parse().ok()).ok_or_else(||de::Error::custom("invalid signed integer"))).transpose()
    }
}

/// Canonicalize arbitrary JSON evidence too: account snapshots and VM registers
/// are sometimes nested in an untyped divergence or RPC result.
pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|n|n>MAX_SAFE_INTEGER) => Value::String(number.to_string()),
        Value::Number(number) if number.as_i64().is_some_and(|n|n.unsigned_abs()>MAX_SAFE_INTEGER) => Value::String(number.to_string()),
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(values) => Value::Object(values.iter().map(|(key,value)|(key.clone(),canonicalize(value))).collect()),
        _ => value.clone(),
    }
}
pub fn serialize_json<T:Serialize,S:Serializer>(value:&T,serializer:S)->Result<S::Ok,S::Error>{
    canonicalize(&serde_json::to_value(value).map_err(serde::ser::Error::custom)?).serialize(serializer)
}
pub fn deserialize_json<'de,D:Deserializer<'de>>(deserializer:D)->Result<Value,D::Error>{
    Value::deserialize(deserializer).map(|value|canonicalize(&value))
}
pub fn deserialize_optional_json<'de,D:Deserializer<'de>>(deserializer:D)->Result<Option<Value>,D::Error>{
    Option::<Value>::deserialize(deserializer).map(|value|value.map(|value|canonicalize(&value)))
}

pub mod array {
    use super::*;
    pub fn serialize<S:Serializer,const N:usize>(values:&[u64;N],serializer:S)->Result<S::Ok,S::Error>{
        values.iter().map(|n|value(*n)).collect::<Vec<_>>().serialize(serializer)
    }
    pub fn deserialize<'de,D:Deserializer<'de>,const N:usize>(deserializer:D)->Result<[u64;N],D::Error>{
        let values=Vec::<Value>::deserialize(deserializer)?.into_iter().map(|v|integer(&v).ok_or_else(||de::Error::custom("invalid unsigned integer"))).collect::<Result<Vec<_>,D::Error>>()?;
        values.try_into().map_err(|_|de::Error::custom("incorrect integer array length"))
    }
}
