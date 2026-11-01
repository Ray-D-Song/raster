// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Process-wide SharedArrayBuffer backing store for QuickJS worker isolates.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use rquickjs::qjs::{self, JSSharedArrayBufferFunctions};

type SabPtr = *mut std::ffi::c_void;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct Block {
    data: Vec<u8>,
    refs: AtomicUsize,
}

static BLOCKS: OnceLock<Mutex<HashMap<u64, Block>>> = OnceLock::new();
static PTR_INDEX: OnceLock<Mutex<HashMap<usize, u64>>> = OnceLock::new();

fn blocks() -> &'static Mutex<HashMap<u64, Block>> {
    BLOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ptr_index() -> &'static Mutex<HashMap<usize, u64>> {
    PTR_INDEX.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe extern "C" fn sab_alloc(_opaque: *mut std::ffi::c_void, size: u64) -> SabPtr {
    let len = size as usize;
    let mut data = Vec::with_capacity(len);
    data.resize(len, 0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let ptr = data.as_mut_ptr() as SabPtr;
    let key = ptr as usize;
    blocks().lock().unwrap().insert(
        id,
        Block {
            data,
            refs: AtomicUsize::new(1),
        },
    );
    ptr_index().lock().unwrap().insert(key, id);
    ptr
}

unsafe extern "C" fn sab_free(_opaque: *mut std::ffi::c_void, ptr: SabPtr) {
    if ptr.is_null() {
        return;
    }
    let key = ptr as usize;
    if let Some(id) = ptr_index().lock().unwrap().remove(&key) {
        blocks().lock().unwrap().remove(&id);
    }
}

unsafe extern "C" fn sab_dup(_opaque: *mut std::ffi::c_void, ptr: SabPtr) {
    if ptr.is_null() {
        return;
    }
    let key = ptr as usize;
    if let Some(id) = ptr_index().lock().unwrap().get(&key) {
        if let Some(block) = blocks().lock().unwrap().get(id) {
            block.refs.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Install process-wide SAB hooks on a QuickJS runtime (call once per `JSRuntime`).
pub unsafe fn install_on_runtime(rt: *mut qjs::JSRuntime) {
    let funcs = JSSharedArrayBufferFunctions {
        sab_alloc: Some(sab_alloc),
        sab_free: Some(sab_free),
        sab_dup: Some(sab_dup),
        sab_opaque: std::ptr::null_mut(),
    };
    qjs::JS_SetSharedArrayBufferFunctions(rt, &funcs);
}

/// Lookup stable id for a SAB data pointer (for structured-clone wire).
pub fn id_for_data_ptr(ptr: *const u8) -> Option<u64> {
    ptr_index().lock().unwrap().get(&(ptr as usize)).copied()
}

/// Dup backing block and return data pointer for decode into another isolate.
pub fn dup_block(id: u64) -> Option<*mut u8> {
    let blocks = blocks().lock().unwrap();
    let block = blocks.get(&id)?;
    block.refs.fetch_add(1, Ordering::Relaxed);
    Some(block.data.as_ptr() as *mut u8)
}

pub fn block_len(id: u64) -> Option<usize> {
    blocks().lock().unwrap().get(&id).map(|b| b.data.len())
}
