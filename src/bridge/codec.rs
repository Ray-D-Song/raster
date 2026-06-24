use std::collections::BTreeMap;
use std::io::Read;

use crate::bridge::envelope::BridgeEnvelope;
use crate::bridge::value::BridgeValue;

const TAG_NULL: u8 = 0;
const TAG_BOOL: u8 = 1;
const TAG_NUMBER: u8 = 2;
const TAG_STRING: u8 = 3;
const TAG_ARRAY: u8 = 4;
const TAG_OBJECT: u8 = 5;
const TAG_BYTES: u8 = 6;

const KIND_CALL: u8 = 1;
const KIND_REPLY: u8 = 2;
const KIND_EVENT: u8 = 3;

pub fn encode_envelope(envelope: &BridgeEnvelope) -> Vec<u8> {
    let mut out = Vec::new();
    match envelope {
        BridgeEnvelope::Call {
            id,
            channel,
            method,
            payload,
        } => {
            out.push(KIND_CALL);
            write_u64(&mut out, *id);
            write_string(&mut out, channel);
            write_string(&mut out, method);
            write_value(&mut out, payload);
        }
        BridgeEnvelope::Reply {
            id,
            ok,
            payload,
            error,
        } => {
            out.push(KIND_REPLY);
            write_u64(&mut out, *id);
            out.push(if *ok { 1 } else { 0 });
            write_value(&mut out, payload);
            write_optional_string(&mut out, error.as_deref());
        }
        BridgeEnvelope::Event {
            channel,
            name,
            payload,
        } => {
            out.push(KIND_EVENT);
            write_string(&mut out, channel);
            write_string(&mut out, name);
            write_value(&mut out, payload);
        }
    }
    out
}

pub fn decode_envelope(bytes: &[u8]) -> anyhow::Result<BridgeEnvelope> {
    let mut cursor = std::io::Cursor::new(bytes);
    let kind = read_u8(&mut cursor)?;
    match kind {
        KIND_CALL => Ok(BridgeEnvelope::Call {
            id: read_u64(&mut cursor)?,
            channel: read_string(&mut cursor)?,
            method: read_string(&mut cursor)?,
            payload: read_value(&mut cursor)?,
        }),
        KIND_REPLY => Ok(BridgeEnvelope::Reply {
            id: read_u64(&mut cursor)?,
            ok: read_u8(&mut cursor)? != 0,
            payload: read_value(&mut cursor)?,
            error: read_optional_string(&mut cursor)?,
        }),
        KIND_EVENT => Ok(BridgeEnvelope::Event {
            channel: read_string(&mut cursor)?,
            name: read_string(&mut cursor)?,
            payload: read_value(&mut cursor)?,
        }),
        other => anyhow::bail!("unknown bridge envelope kind: {other}"),
    }
}

fn write_value(out: &mut Vec<u8>, value: &BridgeValue) {
    match value {
        BridgeValue::Null => out.push(TAG_NULL),
        BridgeValue::Bool(value) => {
            out.push(TAG_BOOL);
            out.push(if *value { 1 } else { 0 });
        }
        BridgeValue::Number(value) => {
            out.push(TAG_NUMBER);
            out.extend_from_slice(&value.to_le_bytes());
        }
        BridgeValue::String(value) => {
            out.push(TAG_STRING);
            write_string(out, value);
        }
        BridgeValue::Array(items) => {
            out.push(TAG_ARRAY);
            write_u32(out, items.len() as u32);
            for item in items {
                write_value(out, item);
            }
        }
        BridgeValue::Object(entries) => {
            out.push(TAG_OBJECT);
            write_u32(out, entries.len() as u32);
            for (key, item) in entries {
                write_string(out, key);
                write_value(out, item);
            }
        }
        BridgeValue::Bytes(bytes) => {
            out.push(TAG_BYTES);
            write_u32(out, bytes.len() as u32);
            out.extend_from_slice(bytes);
        }
    }
}

fn read_value(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<BridgeValue> {
    match read_u8(cursor)? {
        TAG_NULL => Ok(BridgeValue::Null),
        TAG_BOOL => Ok(BridgeValue::Bool(read_u8(cursor)? != 0)),
        TAG_NUMBER => {
            let mut buf = [0u8; 8];
            cursor.read_exact(&mut buf)?;
            Ok(BridgeValue::Number(f64::from_le_bytes(buf)))
        }
        TAG_STRING => Ok(BridgeValue::String(read_string(cursor)?)),
        TAG_ARRAY => {
            let len = read_u32(cursor)? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_value(cursor)?);
            }
            Ok(BridgeValue::Array(items))
        }
        TAG_OBJECT => {
            let len = read_u32(cursor)? as usize;
            let mut entries = BTreeMap::new();
            for _ in 0..len {
                let key = read_string(cursor)?;
                entries.insert(key, read_value(cursor)?);
            }
            Ok(BridgeValue::Object(entries))
        }
        TAG_BYTES => {
            let len = read_u32(cursor)? as usize;
            let mut bytes = vec![0u8; len];
            cursor.read_exact(&mut bytes)?;
            Ok(BridgeValue::Bytes(bytes))
        }
        tag => anyhow::bail!("unknown bridge value tag: {tag}"),
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            out.push(1);
            write_string(out, value);
        }
        None => out.push(0),
    }
}

fn read_u8(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<u8> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<u32> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<u64> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_string(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<String> {
    let len = read_u32(cursor)? as usize;
    let mut bytes = vec![0u8; len];
    cursor.read_exact(&mut bytes)?;
    Ok(String::from_utf8(bytes)?)
}

fn read_optional_string(cursor: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<Option<String>> {
    if read_u8(cursor)? == 0 {
        return Ok(None);
    }
    Ok(Some(read_string(cursor)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_call_with_bytes() {
        let envelope = BridgeEnvelope::Call {
            id: 7,
            channel: "host.assets".to_owned(),
            method: "load".to_owned(),
            payload: BridgeValue::object([
                ("uri", BridgeValue::string("https://example.com/avatar.png")),
                ("bytes", BridgeValue::Bytes(vec![1, 2, 3, 4])),
            ]),
        };
        let encoded = encode_envelope(&envelope);
        let decoded = decode_envelope(&encoded).expect("decode");
        assert_eq!(decoded, envelope);
    }
}