use std::fmt;
use anyhow::Result;
use serde::{Deserialize, Deserializer, de::{self, MapAccess, SeqAccess, Visitor}};
use serde_json::{Map, Number, Value};

struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("finite JSON with unique object fields") }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> { Ok(Unique(Value::Null)) }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Unique, E> { Ok(Unique(Value::Null)) }
            fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Unique, E> { Ok(Unique(value.into())) }
            fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Unique, E> { Ok(Unique(value.into())) }
            fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Unique, E> { Ok(Unique(value.into())) }
            fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Unique, E> {
                Number::from_f64(value).map(|n| Unique(Value::Number(n))).ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Unique, E> { Ok(Unique(value.into())) }
            fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Unique, E> { Ok(Unique(value.into())) }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? { values.push(value); }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) { return Err(de::Error::custom(format!("duplicate JSON field: {key}"))); }
                    let Unique(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

pub fn parse(bytes: &[u8]) -> Result<Value> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let Unique(value) = Unique::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}
