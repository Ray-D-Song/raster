// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::io::{Read, Write};

use raster_runtime_utils::clone_wire::CloneWirePayload;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const FRAME_BOOTSTRAP: u8 = 1;
pub const FRAME_ONLINE: u8 = 2;
pub const FRAME_MESSAGE: u8 = 3;
pub const FRAME_ERROR: u8 = 4;
pub const FRAME_EXIT: u8 = 5;
pub const FRAME_ENV_PATCH: u8 = 6;

pub fn write_frame_sync(writer: &mut impl Write, tag: u8, payload: &CloneWirePayload) -> std::io::Result<()> {
    write_frame_inner(writer, tag, payload)
}

pub async fn write_frame_async(
    writer: &mut (impl AsyncWriteExt + Unpin),
    tag: u8,
    payload: &CloneWirePayload,
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    write_frame_inner(&mut buf, tag, payload)?;
    writer.write_all(&buf).await
}

fn write_frame_inner(writer: &mut impl Write, tag: u8, payload: &CloneWirePayload) -> std::io::Result<()> {
    writer.write_all(&[tag])?;
    write_u32(writer, payload.body.len() as u32)?;
    writer.write_all(&payload.body)?;
    write_u32(writer, payload.transfers.len() as u32)?;
    for t in &payload.transfers {
        write_u32(writer, t.len() as u32)?;
        writer.write_all(t)?;
    }
    writer.flush()
}

pub fn read_frame_sync(reader: &mut impl Read) -> std::io::Result<(u8, CloneWirePayload)> {
    let mut tag = [0u8; 1];
    reader.read_exact(&mut tag)?;
    let body_len = read_u32_sync(reader)? as usize;
    let mut body = vec![0u8; body_len];
    reader.read_exact(&mut body)?;
    let transfer_count = read_u32_sync(reader)? as usize;
    let mut transfers = Vec::with_capacity(transfer_count);
    for _ in 0..transfer_count {
        let len = read_u32_sync(reader)? as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        transfers.push(bytes);
    }
    Ok((tag[0], CloneWirePayload { body, transfers }))
}

pub async fn read_frame_async(
    reader: &mut (impl AsyncReadExt + Unpin),
) -> std::io::Result<(u8, CloneWirePayload)> {
    let mut tag = [0u8; 1];
    reader.read_exact(&mut tag).await?;
    let body_len = read_u32_async(reader).await? as usize;
    let mut body = vec![0u8; body_len];
    reader.read_exact(&mut body).await?;
    let transfer_count = read_u32_async(reader).await? as usize;
    let mut transfers = Vec::with_capacity(transfer_count);
    for _ in 0..transfer_count {
        let len = read_u32_async(reader).await? as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes).await?;
        transfers.push(bytes);
    }
    Ok((tag[0], CloneWirePayload { body, transfers }))
}

pub fn bootstrap_payload(worker: CloneWirePayload, env: CloneWirePayload) -> CloneWirePayload {
    let mut body = Vec::new();
    write_u32_vec(&mut body, &worker.body);
    write_u32_vec(&mut body, &env.body);
    let mut transfers = worker.transfers;
    transfers.extend(env.transfers);
    CloneWirePayload { body, transfers }
}

pub fn split_bootstrap(payload: &CloneWirePayload) -> std::io::Result<(CloneWirePayload, CloneWirePayload)> {
    let mut pos = 0;
    let worker_body = read_u32_vec(&payload.body, &mut pos)?;
    let env_body = read_u32_vec(&payload.body, &mut pos)?;
    Ok((
        CloneWirePayload {
            body: worker_body,
            transfers: payload.transfers.clone(),
        },
        CloneWirePayload {
            body: env_body,
            transfers: Vec::new(),
        },
    ))
}

fn write_u32(writer: &mut impl Write, v: u32) -> std::io::Result<()> {
    writer.write_all(&v.to_be_bytes())
}

fn read_u32_sync(reader: &mut impl Read) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u32_async(reader: &mut (impl AsyncReadExt + Unpin)) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).await?;
    Ok(u32::from_be_bytes(buf))
}

fn write_u32_vec(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn read_u32_vec(data: &[u8], pos: &mut usize) -> std::io::Result<Vec<u8>> {
    if *pos + 4 > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "bootstrap truncated",
        ));
    }
    let len = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap()) as usize;
    *pos += 4;
    if *pos + len > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "bootstrap truncated",
        ));
    }
    let slice = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(slice)
}
