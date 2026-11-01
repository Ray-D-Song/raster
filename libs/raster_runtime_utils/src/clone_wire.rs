// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Structured-clone wire encoding for cross-process worker IPC (binary, not JSON).

use std::cell::RefCell;
use std::collections::HashMap;

use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, This},
    qjs::JS_VALUE_GET_PTR,
    Array, ArrayBuffer, Ctx, Exception, Function, IntoJs, Null, Object, Result, Type, Undefined,
    Value,
};

use crate::clone::structured_clone;
use crate::primordials::{BasePrimordials, Primordial};

pub const MAGIC: &[u8; 4] = b"RSC1";

#[derive(Clone, Debug)]
enum WireNode {
    Ref(u32),
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    BigInt(String),
    Array(Vec<u32>),
    Object(Vec<(String, u32)>),
    Date(f64),
    RegExp { source: String, flags: String },
    Error(String),
    ArrayBufferCopy(Vec<u8>),
    Map(Vec<(u32, u32)>),
    Set(Vec<u32>),
}

#[derive(Debug, Default, Clone)]
pub struct CloneWirePayload {
    pub body: Vec<u8>,
    pub transfers: Vec<Vec<u8>>,
}

pub fn serialize<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    transfer: Opt<Object<'js>>,
) -> Result<CloneWirePayload> {
    let cloned = structured_clone(ctx, value, transfer)?;
    let nodes: RefCell<Vec<WireNode>> = RefCell::new(Vec::new());
    let mut memo: HashMap<usize, u32> = HashMap::new();
    let mut transfers = Vec::new();
    let root = encode_index(ctx, &cloned, &nodes, &mut memo, &mut transfers)?;
    let nodes = nodes.into_inner();
    let mut body = Vec::new();
    body.extend_from_slice(MAGIC);
    write_u32(&mut body, root);
    write_u32(&mut body, nodes.len() as u32);
    for node in &nodes {
        write_node(&mut body, node);
    }
    Ok(CloneWirePayload { body, transfers })
}

pub fn deserialize<'js>(ctx: &Ctx<'js>, payload: &CloneWirePayload) -> Result<Value<'js>> {
    if payload.body.len() < 4 || &payload.body[0..4] != MAGIC {
        return Err(Exception::throw_type(ctx, "Invalid structured-clone wire payload"));
    }
    let mut pos = 4;
    let root = read_u32(&payload.body, &mut pos)?;
    let count = read_u32(&payload.body, &mut pos)? as usize;
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        nodes.push(read_node(&payload.body, &mut pos)?);
    }
    decode_index(ctx, &nodes, &payload.transfers, root)
}

fn object_ptr(value: &Value<'_>) -> Option<usize> {
    if !matches!(value.type_of(), Type::Object | Type::Array) {
        return None;
    }
    Some(unsafe { JS_VALUE_GET_PTR(value.as_raw()) as usize })
}

fn encode_index<'js>(
    ctx: &Ctx<'js>,
    value: &Value<'js>,
    nodes: &RefCell<Vec<WireNode>>,
    memo: &mut HashMap<usize, u32>,
    transfers: &mut Vec<Vec<u8>>,
) -> Result<u32> {
    if let Some(ptr) = object_ptr(value) {
        if let Some(id) = memo.get(&ptr) {
            return Ok(*id);
        }
        let idx = nodes.borrow().len() as u32;
        memo.insert(ptr, idx);
        nodes.borrow_mut().push(WireNode::Null); // placeholder for cycles
        let node = encode_node_inner(ctx, value, nodes, memo, transfers)?;
        nodes.borrow_mut()[idx as usize] = node;
        return Ok(idx);
    }
    let idx = nodes.borrow().len() as u32;
    nodes.borrow_mut().push(encode_node_inner(ctx, value, nodes, memo, transfers)?);
    Ok(idx)
}

fn encode_node_inner<'js>(
    ctx: &Ctx<'js>,
    value: &Value<'js>,
    nodes: &RefCell<Vec<WireNode>>,
    memo: &mut HashMap<usize, u32>,
    transfers: &mut Vec<Vec<u8>>,
) -> Result<WireNode> {
    let primordials = BasePrimordials::get(ctx)?;
    match value.type_of() {
        Type::Undefined => Ok(WireNode::Undefined),
        Type::Null => Ok(WireNode::Null),
        Type::Bool => Ok(WireNode::Bool(value.as_bool().unwrap_or(false))),
        Type::Int | Type::Float => Ok(WireNode::Number(value.as_number().unwrap_or(f64::NAN))),
        Type::String => Ok(WireNode::String(
            value.as_string().unwrap().to_string()?,
        )),
        Type::BigInt => Err(Exception::throw_type(
            ctx,
            "BigInt cannot be cloned across worker processes yet",
        )),
        Type::Array => {
            let array = unsafe { value.as_array().unwrap_unchecked() };
            let mut items = Vec::with_capacity(array.len());
            for i in 0..array.len() {
                let child = array.get(i)?;
                let id = encode_index(ctx, &child, nodes, memo, transfers)?;
                items.push(id);
            }
            Ok(WireNode::Array(items))
        },
        Type::Object => {
            let object = unsafe { value.as_object().unwrap_unchecked() };
            if object.is_instance_of(&primordials.constructor_date) {
                let get_time: Function = object.get("getTime")?;
                let time: f64 = get_time.call::<_, f64>((This(object.clone()),))?;
                return Ok(WireNode::Date(time));
            }
            if object.is_instance_of(&primordials.constructor_regexp) {
                return Ok(WireNode::RegExp {
                    source: object.get("source")?,
                    flags: object.get("flags")?,
                });
            }
            if object.is_instance_of(&primordials.constructor_error) {
                let message: String = object
                    .get(PredefinedAtom::Message)
                    .unwrap_or_else(|_| String::new());
                return Ok(WireNode::Error(message));
            }
            if let Some(ab) = ArrayBuffer::from_value(value.clone()) {
                if is_shared_array_buffer(ctx, &ab)? {
                    return Err(Exception::throw_type(
                        ctx,
                        "SharedArrayBuffer cannot be cloned across worker processes",
                    ));
                }
                let slice: Function = ab.get("slice")?;
                let sliced: Value = slice.call::<_, Value>((This(ab.clone()),))?;
                let bytes_obj = sliced
                    .as_object()
                    .ok_or_else(|| Exception::throw_type(ctx, "Invalid ArrayBuffer"))?;
                let bytes_obj = ArrayBuffer::from_object(bytes_obj.clone())
                    .ok_or_else(|| Exception::throw_type(ctx, "Invalid ArrayBuffer"))?;
                let bytes = read_array_buffer_bytes(ctx, &bytes_obj)?;
                transfers.push(bytes.clone());
                return Ok(WireNode::ArrayBufferCopy(bytes));
            }
            if object.is_instance_of(&primordials.constructor_map) {
                let entries: Array = primordials
                    .function_array_from
                    .call((object.clone(),))?;
                let mut pairs = Vec::new();
                for entry in entries.iter::<Array>() {
                    let entry = entry?;
                    let k = encode_index(ctx, &entry.get(0)?, nodes, memo, transfers)?;
                    let v = encode_index(ctx, &entry.get(1)?, nodes, memo, transfers)?;
                    pairs.push((k, v));
                }
                return Ok(WireNode::Map(pairs));
            }
            if object.is_instance_of(&primordials.constructor_set) {
                let entries: Array = primordials
                    .function_array_from
                    .call((object.clone(),))?;
                let mut items = Vec::new();
                for item in entries.iter::<Value>() {
                    items.push(encode_index(ctx, &item?, nodes, memo, transfers)?);
                }
                return Ok(WireNode::Set(items));
            }
            if primordials
                .function_array_buffer_is_view
                .call::<_, bool>((value.clone(),))?
            {
                let bytes: Object = object.get("buffer")?;
                let ab = ArrayBuffer::from_object(bytes)
                    .ok_or_else(|| Exception::throw_type(ctx, "Invalid typed array"))?;
                if is_shared_array_buffer(ctx, &ab)? {
                    return Err(Exception::throw_type(
                        ctx,
                        "SharedArrayBuffer cannot be cloned across worker processes",
                    ));
                }
                let bytes = read_array_buffer_bytes(ctx, &ab)?;
                transfers.push(bytes.clone());
                return Ok(WireNode::ArrayBufferCopy(bytes));
            }
            let keys: Vec<String> = object.keys().collect::<Result<_>>()?;
            let mut props = Vec::with_capacity(keys.len());
            for key in keys {
                let v: Value = object.get(&key)?;
                props.push((key, encode_index(ctx, &v, nodes, memo, transfers)?));
            }
            Ok(WireNode::Object(props))
        },
        Type::Function | Type::Constructor | Type::Proxy => Err(Exception::throw_type(
            ctx,
            "The object could not be cloned.",
        )),
        _ => Ok(WireNode::Undefined),
    }
}

fn decode_index<'js>(
    ctx: &Ctx<'js>,
    nodes: &[WireNode],
    transfers: &[Vec<u8>],
    idx: u32,
) -> Result<Value<'js>> {
    let mut cache: Vec<Option<Value<'js>>> = vec![None; nodes.len()];
    decode_at(ctx, nodes, transfers, idx as usize, &mut cache)
}

fn decode_at<'js>(
    ctx: &Ctx<'js>,
    nodes: &[WireNode],
    transfers: &[Vec<u8>],
    idx: usize,
    cache: &mut [Option<Value<'js>>],
) -> Result<Value<'js>> {
    if let Some(v) = &cache[idx] {
        return Ok(v.clone());
    }
    let value = match &nodes[idx] {
        WireNode::Ref(id) => return decode_at(ctx, nodes, transfers, *id as usize, cache),
        WireNode::Undefined => Undefined.into_value(ctx.clone()),
        WireNode::Null => Null.into_value(ctx.clone()),
        WireNode::Bool(b) => b.into_js(ctx)?,
        WireNode::Number(n) => n.into_js(ctx)?,
        WireNode::String(s) => s.clone().into_js(ctx)?,
        WireNode::BigInt(s) => {
            let s = s.replace('\\', "\\\\").replace('\'', "\\'");
            let code = format!("BigInt('{s}')");
            ctx.eval(code)?
        },
        WireNode::Date(time) => {
            let primordials = BasePrimordials::get(ctx)?;
            let obj: Object = primordials.constructor_date.construct((*time,))?;
            obj.into_js(ctx)?
        },
        WireNode::RegExp { source, flags } => {
            let primordials = BasePrimordials::get(ctx)?;
            let obj: Object =
                primordials
                    .constructor_regexp
                    .construct((source.clone(), flags.clone()))?;
            obj.into_js(ctx)?
        },
        WireNode::Error(message) => {
            let primordials = BasePrimordials::get(ctx)?;
            let obj: Object = primordials
                .constructor_error
                .construct((message.clone(),))?;
            obj.into_js(ctx)?
        },
        WireNode::ArrayBufferCopy(bytes) => {
            let buf = if let Some(t) = transfers.first() {
                t.clone()
            } else {
                bytes.clone()
            };
            ArrayBuffer::new_copy(ctx.clone(), &buf)?.into_js(ctx)?
        },
        WireNode::Array(indices) => {
            let array = Array::new(ctx.clone())?;
            cache[idx] = Some(array.clone().into_js(ctx)?);
            for (i, child) in indices.iter().enumerate() {
                let v = decode_at(ctx, nodes, transfers, *child as usize, cache)?;
                array.set(i, v)?;
            }
            array.into_js(ctx)?
        },
        WireNode::Object(props) => {
            let object = Object::new(ctx.clone())?;
            cache[idx] = Some(object.clone().into_js(ctx)?);
            for (key, child) in props {
                let v = decode_at(ctx, nodes, transfers, *child as usize, cache)?;
                object.set(key.clone(), v)?;
            }
            object.into_js(ctx)?
        },
        WireNode::Map(pairs) => {
            let primordials = BasePrimordials::get(ctx)?;
            let entries = Array::new(ctx.clone())?;
            for (k, v) in pairs {
                let pair = Array::new(ctx.clone())?;
                pair.set(0, decode_at(ctx, nodes, transfers, *k as usize, cache)?)?;
                pair.set(1, decode_at(ctx, nodes, transfers, *v as usize, cache)?)?;
                entries.set(entries.len(), pair)?;
            }
            let obj: Object = primordials.constructor_map.construct((entries,))?;
            obj.into_js(ctx)?
        },
        WireNode::Set(items) => {
            let primordials = BasePrimordials::get(ctx)?;
            let entries = Array::new(ctx.clone())?;
            for child in items {
                entries.set(
                    entries.len(),
                    decode_at(ctx, nodes, transfers, *child as usize, cache)?,
                )?;
            }
            let obj: Object = primordials.constructor_set.construct((entries,))?;
            obj.into_js(ctx)?
        },
    };
    if cache[idx].is_none() {
        cache[idx] = Some(value.clone());
    }
    Ok(value)
}

fn read_array_buffer_bytes<'js>(_ctx: &Ctx<'js>, ab: &ArrayBuffer<'js>) -> Result<Vec<u8>> {
    Ok(ab.as_bytes().map(|s| s.to_vec()).unwrap_or_default())
}

fn is_shared_array_buffer<'js>(ctx: &Ctx<'js>, ab: &ArrayBuffer<'js>) -> Result<bool> {
    let ctor: Object = ab.get(PredefinedAtom::Constructor)?;
    let name: String = ctor.get(PredefinedAtom::Name)?;
    Ok(name == "SharedArrayBuffer")
}

fn write_node(out: &mut Vec<u8>, node: &WireNode) {
    match node {
        WireNode::Ref(id) => {
            out.push(1);
            write_u32(out, *id);
        },
        WireNode::Undefined => out.push(2),
        WireNode::Null => out.push(3),
        WireNode::Bool(b) => {
            out.push(4);
            out.push(u8::from(*b));
        },
        WireNode::Number(n) => {
            out.push(5);
            write_f64(out, *n);
        },
        WireNode::String(s) => {
            out.push(6);
            write_bytes(out, s.as_bytes());
        },
        WireNode::BigInt(s) => {
            out.push(7);
            write_bytes(out, s.as_bytes());
        },
        WireNode::Array(items) => {
            out.push(8);
            write_u32(out, items.len() as u32);
            for id in items {
                write_u32(out, *id);
            }
        },
        WireNode::Object(props) => {
            out.push(9);
            write_u32(out, props.len() as u32);
            for (k, id) in props {
                write_bytes(out, k.as_bytes());
                write_u32(out, *id);
            }
        },
        WireNode::Date(t) => {
            out.push(10);
            write_f64(out, *t);
        },
        WireNode::RegExp { source, flags } => {
            out.push(11);
            write_bytes(out, source.as_bytes());
            write_bytes(out, flags.as_bytes());
        },
        WireNode::Error(msg) => {
            out.push(12);
            write_bytes(out, msg.as_bytes());
        },
        WireNode::ArrayBufferCopy(bytes) => {
            out.push(13);
            write_bytes(out, bytes);
        },
        WireNode::Map(pairs) => {
            out.push(14);
            write_u32(out, pairs.len() as u32);
            for (k, v) in pairs {
                write_u32(out, *k);
                write_u32(out, *v);
            }
        },
        WireNode::Set(items) => {
            out.push(15);
            write_u32(out, items.len() as u32);
            for id in items {
                write_u32(out, *id);
            }
        },
    }
}

fn read_node(data: &[u8], pos: &mut usize) -> Result<WireNode> {
    let tag = read_u8(data, pos)?;
    Ok(match tag {
        1 => WireNode::Ref(read_u32(data, pos)?),
        2 => WireNode::Undefined,
        3 => WireNode::Null,
        4 => WireNode::Bool(read_u8(data, pos)? != 0),
        5 => WireNode::Number(read_f64(data, pos)?),
        6 => WireNode::String(read_string(data, pos)?),
        7 => WireNode::BigInt(read_string(data, pos)?),
        8 => {
            let n = read_u32(data, pos)? as usize;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(read_u32(data, pos)?);
            }
            WireNode::Array(items)
        },
        9 => {
            let n = read_u32(data, pos)? as usize;
            let mut props = Vec::with_capacity(n);
            for _ in 0..n {
                props.push((read_string(data, pos)?, read_u32(data, pos)?));
            }
            WireNode::Object(props)
        },
        10 => WireNode::Date(read_f64(data, pos)?),
        11 => WireNode::RegExp {
            source: read_string(data, pos)?,
            flags: read_string(data, pos)?,
        },
        12 => WireNode::Error(read_string(data, pos)?),
        13 => WireNode::ArrayBufferCopy(read_vec(data, pos)?),
        14 => {
            let n = read_u32(data, pos)? as usize;
            let mut pairs = Vec::with_capacity(n);
            for _ in 0..n {
                pairs.push((read_u32(data, pos)?, read_u32(data, pos)?));
            }
            WireNode::Map(pairs)
        },
        15 => {
            let n = read_u32(data, pos)? as usize;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(read_u32(data, pos)?);
            }
            WireNode::Set(items)
        },
        _ => return Err(rquickjs::Error::new_from_js_message(
            "clone_wire",
            "decode",
            "Unknown wire node tag",
        )),
    })
}

fn read_string(data: &[u8], pos: &mut usize) -> Result<String> {
    let bytes = read_vec(data, pos)?;
    String::from_utf8(bytes).map_err(|_| {
        rquickjs::Error::new_from_js_message("clone_wire", "decode", "Invalid UTF-8")
    })
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8> {
    if *pos >= data.len() {
        return Err(rquickjs::Error::new_from_js_message(
            "clone_wire",
            "decode",
            "Unexpected end of wire",
        ));
    }
    let b = data[*pos];
    *pos += 1;
    Ok(b)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > data.len() {
        return Err(rquickjs::Error::new_from_js_message(
            "clone_wire",
            "decode",
            "Unexpected end of wire",
        ));
    }
    let v = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_f64(data: &[u8], pos: &mut usize) -> Result<f64> {
    if *pos + 8 > data.len() {
        return Err(rquickjs::Error::new_from_js_message(
            "clone_wire",
            "decode",
            "Unexpected end of wire",
        ));
    }
    let v = f64::from_be_bytes(data[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(v)
}

fn read_vec(data: &[u8], pos: &mut usize) -> Result<Vec<u8>> {
    let len = read_u32(data, pos)? as usize;
    if *pos + len > data.len() {
        return Err(rquickjs::Error::new_from_js_message(
            "clone_wire",
            "decode",
            "Unexpected end of wire",
        ));
    }
    let slice = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use raster_runtime_test::test_sync_with;
    use rquickjs::{function::Opt, Object, Value};

    use crate::primordials::{BasePrimordials, Primordial};

    use super::{deserialize, serialize};

    #[tokio::test]
    async fn roundtrip_object() {
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
            let value: Object = ctx.eval(
                r#"
({ n: 1, nested: { a: "b" }, arr: [1,2,3] })
"#,
            )?;
            let wire = serialize(&ctx, value.into_value(), Opt(None))?;
            let back = deserialize(&ctx, &wire)?;
            let _: Value = back;
            Ok(())
        })
        .await;
    }
}
