//! Deserialize before conversion to a JSON object: Value loaders can overwrite
//! repeated YAML mapping entries before an importer ever sees them.
use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};

use crate::{ImportError, Result};

struct Unique(Value);
struct StringKey(String);
impl<'de> Deserialize<'de> for StringKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = StringKey;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("a string mapping key") }
            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<StringKey, E> { Ok(StringKey(value.into())) }
            fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<StringKey, E> { Ok(StringKey(value)) }
        }
        deserializer.deserialize_any(KeyVisitor)
    }
}
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a finite YAML value with unique string mapping keys")
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> { Ok(Unique(Value::Null)) }
    fn visit_none<E: de::Error>(self) -> std::result::Result<Unique, E> { self.visit_unit() }
    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
        Number::from_f64(v).map(|n| Unique(Value::Number(n)))
            .ok_or_else(|| E::custom("non-finite YAML number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Unique, A::Error> {
        let mut values = Vec::new();
        while let Some(v) = seq.next_element::<Unique>()? { values.push(v.0); }
        Ok(Unique(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Unique, A::Error> {
        let mut values = Map::new();
        while let Some(StringKey(key)) = map.next_key::<StringKey>()? {
            if key == "<<" { return Err(de::Error::custom("YAML merge keys are ambiguous and unsupported")); }
            if values.contains_key(&key) { return Err(de::Error::custom(format!("duplicate YAML key {key:?}"))); }
            values.insert(key, map.next_value::<Unique>()?.0);
        }
        Ok(Unique(Value::Object(values)))
    }
}

pub(crate) fn parse(text: &str) -> Result<Value> {
    let mut documents = serde_yaml::Deserializer::from_str(text);
    let document = documents.next().ok_or_else(|| ImportError::at("/", "empty USTX document"))?;
    let value = Unique::deserialize(document).map_err(ImportError::Yaml)?.0;
    if documents.next().is_some() { return Err(ImportError::at("/", "multiple YAML documents are unsupported")); }
    Ok(value)
}

pub(crate) fn parse_json(bytes: &[u8]) -> Result<Value> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Unique::deserialize(&mut deserializer)?.0;
    deserializer.end()?;
    Ok(value)
}
