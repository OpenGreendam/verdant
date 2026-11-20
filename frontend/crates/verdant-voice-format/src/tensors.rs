//! Bounded safetensors transport for exact voice records and token indexes.
//! Float records use their original little-endian F64 bits. Token bytes and
//! parsed indexes share the same complete, non-overlapping data inventory.
use std::collections::BTreeMap;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use crate::{MAX_ARTIFACT_BYTES, parse_json};

const MAX_HEADER: usize = 8 * 1024 * 1024;
const MAX_TENSORS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dtype { F64, I64, U8 }
impl Dtype {
    fn name(self) -> &'static str { match self { Self::F64 => "F64", Self::I64 => "I64", Self::U8 => "U8" } }
    fn width(self) -> usize { match self { Self::F64 | Self::I64 => 8, Self::U8 => 1 } }
    fn parse(name: &str) -> Result<Self> {
        match name { "F64" => Ok(Self::F64), "I64" => Ok(Self::I64), "U8" => Ok(Self::U8),
            _ => anyhow::bail!("unsupported merged voice tensor dtype") }
    }
}

pub struct Tensor<'a> { pub dtype: Dtype, pub shape: Vec<usize>, pub bytes: &'a [u8] }
pub struct Tensors<'a> { pub metadata: BTreeMap<String, String>, pub tensors: BTreeMap<String, Tensor<'a>> }

fn count(shape: &[usize]) -> Result<usize> {
    ensure!(!shape.is_empty() && shape.len() <= 2, "merged voice tensors require rank one or two");
    shape.iter().try_fold(1usize, |count, &dimension| {
        ensure!(dimension > 0, "merged voice tensor dimensions must be positive");
        count.checked_mul(dimension).context("merged voice tensor size overflow")
    })
}

pub fn decode(bytes: &[u8]) -> Result<Tensors<'_>> {
    ensure!((10..=MAX_ARTIFACT_BYTES).contains(&bytes.len()), "invalid safetensors byte length");
    let size = usize::try_from(u64::from_le_bytes(bytes[..8].try_into()?))?;
    ensure!((2..=MAX_HEADER).contains(&size) && size <= bytes.len() - 8, "safetensors header length is invalid");
    let Value::Object(header) = parse_json(&bytes[8..8 + size])? else { anyhow::bail!("safetensors header must be an object"); };
    ensure!((1..=MAX_TENSORS + 1).contains(&header.len()), "safetensors inventory exceeds its bound");
    let mut metadata = BTreeMap::new();
    let mut tensors = BTreeMap::new();
    let mut ranges = Vec::new();
    let body = &bytes[8 + size..];
    for (name, entry) in header {
        if name == "__metadata__" {
            metadata = serde_json::from_value(entry).context("safetensors metadata must contain only string values")?;
            continue;
        }
        ensure!(!name.is_empty() && name.len() <= 256 && !name.contains('\0'), "invalid merged voice tensor name");
        let entry = entry.as_object().context("safetensors tensor descriptor must be an object")?;
        ensure!(entry.len() == 3 && ["dtype", "shape", "data_offsets"].iter().all(|key| entry.contains_key(*key)),
            "unexpected safetensors tensor fields");
        let dtype = Dtype::parse(entry["dtype"].as_str().context("tensor dtype must be a string")?)?;
        let shape = entry["shape"].as_array().context("tensor shape must be an array")?.iter()
            .map(|value| Ok(usize::try_from(value.as_u64().context("tensor shape must contain nonnegative integers")?)?))
            .collect::<Result<Vec<_>>>()?;
        let wanted = count(&shape)?.checked_mul(dtype.width()).context("tensor size overflow")?;
        let offsets = entry["data_offsets"].as_array().context("tensor offsets must be an array")?;
        ensure!(offsets.len() == 2, "tensor needs two data offsets");
        let start = usize::try_from(offsets[0].as_u64().context("invalid tensor start offset")?)?;
        let end = usize::try_from(offsets[1].as_u64().context("invalid tensor end offset")?)?;
        ensure!(end <= body.len() && end.checked_sub(start) == Some(wanted), "tensor shape and byte range disagree");
        ranges.push((start, end));
        tensors.insert(name, Tensor { dtype, shape, bytes: &body[start..end] });
    }
    ensure!(!tensors.is_empty() && tensors.len() <= MAX_TENSORS, "empty or oversized safetensors inventory");
    ranges.sort_unstable();
    let mut cursor = 0;
    for (start, end) in ranges {
        ensure!(start == cursor && end > start, "overlapping or uncovered safetensors data");
        cursor = end;
    }
    ensure!(cursor == body.len(), "safetensors has unclaimed trailing data");
    Ok(Tensors { metadata, tensors })
}

pub enum Data<'a> { F64(&'a [f64]), I64(&'a [i64]), U8(&'a [u8]) }
impl Data<'_> {
    fn dtype(&self) -> Dtype { match self { Self::F64(_) => Dtype::F64, Self::I64(_) => Dtype::I64, Self::U8(_) => Dtype::U8 } }
    fn len(&self) -> usize { match self { Self::F64(values) => values.len(), Self::I64(values) => values.len(), Self::U8(values) => values.len() } }
}
pub struct Input<'a> { pub name: &'a str, pub shape: Vec<usize>, pub data: Data<'a> }

/// Build the header first, then serialize each borrowed source directly into
/// the final allocation; no extra copy of the F64 model is needed.
pub fn encode(metadata: &BTreeMap<String, String>, inputs: &[Input<'_>]) -> Result<Vec<u8>> {
    ensure!((1..=MAX_TENSORS).contains(&inputs.len()), "invalid tensor inventory");
    let mut header = serde_json::Map::new();
    header.insert("__metadata__".into(), serde_json::to_value(metadata)?);
    let mut cursor = 0usize;
    for input in inputs {
        ensure!(!input.name.is_empty() && input.name != "__metadata__" && input.name.len() <= 256 && !input.name.contains('\0'),
            "invalid merged voice tensor name");
        ensure!(count(&input.shape)? == input.data.len(), "tensor shape and source length disagree");
        let size = input.data.len().checked_mul(input.data.dtype().width()).context("tensor size overflow")?;
        let end = cursor.checked_add(size).context("tensor offset overflow")?;
        ensure!(end <= MAX_ARTIFACT_BYTES, "merged bank exceeds its byte bound");
        let prior = header.insert(input.name.into(), json!({"dtype":input.data.dtype().name(),"shape":input.shape,"data_offsets":[cursor,end]}));
        ensure!(prior.is_none(), "duplicate merged tensor name");
        cursor = end;
    }
    let mut header = serde_json::to_vec(&Value::Object(header))?;
    while header.len() % 8 != 0 { header.push(b' '); }
    ensure!(header.len() <= MAX_HEADER && 8 + header.len() + cursor <= MAX_ARTIFACT_BYTES, "merged bank header or data exceeds its bound");
    let mut result = Vec::with_capacity(8 + header.len() + cursor);
    result.extend_from_slice(&(header.len() as u64).to_le_bytes());
    result.extend_from_slice(&header);
    for input in inputs {
        match &input.data {
            Data::U8(values) => result.extend_from_slice(values),
            Data::I64(values) => for value in *values { result.extend_from_slice(&value.to_le_bytes()); },
            Data::F64(values) => for value in *values {
                ensure!(value.is_finite(), "nonfinite F64 voice record");
                result.extend_from_slice(&value.to_le_bytes());
            },
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_tensor_roundtrip_retains_f64_bits_raw_bytes_and_i64_indexes() {
        let floats = [-0., 1e-250, 2.5];
        let indexes = [-1, 1, i64::MAX];
        let input = [Input { name:"leaves",shape:vec![3],data:Data::F64(&floats) },
            Input { name:"nodes",shape:vec![3],data:Data::I64(&indexes) },
            Input { name:"token",shape:vec![3],data:Data::U8(b"abc") }];
        let bytes = encode(&BTreeMap::from([("schema".into(),"test/1".into())]), &input).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.metadata["schema"], "test/1");
        for (actual, expected) in decoded.tensors["leaves"].bytes.chunks_exact(8).zip(floats) {
            assert_eq!(f64::from_le_bytes(actual.try_into().unwrap()).to_bits(), expected.to_bits());
        }
        assert_eq!(decoded.tensors["token"].bytes, b"abc");
        assert_eq!(decoded.tensors["nodes"].dtype, Dtype::I64);
    }

    #[test]
    fn tensor_transport_rejects_overlap_trailing_bytes_and_duplicate_fields() {
        let original = encode(&BTreeMap::new(), &[Input { name:"a",shape:vec![2],data:Data::U8(b"ab") }]).unwrap();
        let mut trailing = original.clone(); trailing.push(0);
        assert!(decode(&trailing).is_err());
        let mut truncated = original; truncated.pop();
        assert!(decode(&truncated).is_err());
        let header = br#"{"a":{"dtype":"U8","shape":[2],"data_offsets":[0,2]},"b":{"dtype":"U8","shape":[2],"data_offsets":[0,2]}}"#;
        let mut overlap = (header.len() as u64).to_le_bytes().to_vec(); overlap.extend_from_slice(header); overlap.extend_from_slice(b"ab");
        assert!(decode(&overlap).is_err());
        let header = br#"{"a":{"dtype":"U8","dtype":"I64","shape":[2],"data_offsets":[0,2]}}"#;
        let mut duplicate = (header.len() as u64).to_le_bytes().to_vec(); duplicate.extend_from_slice(header); duplicate.extend_from_slice(b"ab");
        assert!(decode(&duplicate).is_err());
    }
}
