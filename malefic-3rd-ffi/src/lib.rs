//! Shared FFI utilities for malefic 3rd-party language modules.
//!
//! Provides common helpers for:
//! - Module name retrieval from FFI
//! - Protobuf request encoding / response decoding (prost)
//! - RAII buffer management for foreign-allocated memory
//! - Re-exports of types needed by FFI module implementations

// ── Re-exports ──────────────────────────────────────────────────────────────
pub use std::ffi::{c_char, c_int, c_uint, CStr};
pub use async_trait::async_trait;
pub use anyhow::anyhow;
pub use malefic_proto::prelude::*;
pub use malefic_proto::proto::modulepb::Request;
pub use prost;
pub use futures;

// ── libc free ───────────────────────────────────────────────────────────────

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

/// Free a pointer allocated by foreign code (C `malloc` / Go `C.CBytes` / etc.).
///
/// # Safety
/// `ptr` must have been allocated by the C allocator (`malloc`).
pub unsafe fn ffi_free(ptr: *mut c_char) {
    free(ptr as *mut std::ffi::c_void);
}

// ── FfiBuffer ───────────────────────────────────────────────────────────────

/// RAII guard for a buffer allocated by foreign code via `malloc`.
///
/// On drop, calls `free()` to release the memory.
/// This prevents leaks when decoding fails or an early return occurs.
pub struct FfiBuffer {
    ptr: *mut c_char,
    len: usize,
}

impl FfiBuffer {
    /// Wrap a foreign-allocated pointer.
    /// # Safety
    /// `ptr` must be non-null, valid for `len` bytes, and allocated via `malloc`.
    pub unsafe fn new(ptr: *mut c_char, len: usize) -> Self {
        Self { ptr, len }
    }

    /// View the buffer contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}

impl Drop for FfiBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi_free(self.ptr) };
        }
    }
}

// ── Module name retrieval ───────────────────────────────────────────────────

/// Retrieve a module name string from an FFI `name_fn`.
///
/// If `needs_free` is true, the returned C string pointer will be freed after
/// copying (Go-style, where the name is heap-allocated via `C.CString`).
/// If false, the pointer is assumed to be static (C/Zig/Nim-style).
///
/// # Safety
/// `name_fn` must return a valid, NUL-terminated C string.
pub unsafe fn ffi_module_name(
    name_fn: unsafe extern "C" fn() -> *const c_char,
    needs_free: bool,
) -> String {
    let ptr = name_fn();
    let name = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    if needs_free {
        ffi_free(ptr as *mut c_char);
    }
    name
}

// ── Protobuf encode / decode ────────────────────────────────────────────────

/// Encode a prost `Request` into bytes suitable for passing across FFI.
pub fn encode_request(request: &Request) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    prost::Message::encode(request, &mut buf)
        .map_err(|e| anyhow!("encode error: {}", e))?;
    Ok(buf)
}

/// Decode a prost `Response` from bytes returned by foreign code.
pub fn decode_response(bytes: &[u8]) -> anyhow::Result<Response> {
    prost::Message::decode(bytes)
        .map_err(|e| anyhow!("decode error: {}", e))
}
