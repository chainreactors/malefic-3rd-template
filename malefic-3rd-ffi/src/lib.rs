//! Shared FFI utilities for malefic 3rd-party language modules (Go, C, etc.).
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

// ── FfiBuffer ───────────────────────────────────────────────────────────────

/// Type alias for a C-side free function.
pub type FreeFn = unsafe extern "C" fn(*mut c_char);

/// RAII guard for a buffer allocated by foreign code (Go / C).
///
/// On drop, calls the provided `free_fn` to release the memory.
/// This prevents leaks when decoding fails or an early return occurs.
pub struct FfiBuffer {
    ptr: *mut c_char,
    len: usize,
    free_fn: FreeFn,
}

impl FfiBuffer {
    /// Wrap a foreign-allocated pointer.
    /// # Safety
    /// `ptr` must be non-null and valid for `len` bytes.
    /// `free_fn` must correctly free `ptr`.
    pub unsafe fn new(ptr: *mut c_char, len: usize, free_fn: FreeFn) -> Self {
        Self { ptr, len, free_fn }
    }

    /// View the buffer contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for FfiBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (self.free_fn)(self.ptr) };
        }
    }
}

// ── Module name retrieval ───────────────────────────────────────────────────

/// Retrieve a module name string from an FFI `name_fn`.
///
/// If `free_fn` is `Some`, the returned C string pointer will be freed after
/// copying (Go-style, where the name is heap-allocated).
/// If `None`, the pointer is assumed to be static (C-style).
///
/// # Safety
/// `name_fn` must return a valid, NUL-terminated C string.
pub unsafe fn ffi_module_name(
    name_fn: unsafe extern "C" fn() -> *const c_char,
    free_fn: Option<FreeFn>,
) -> String {
    let ptr = name_fn();
    let name = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    if let Some(f) = free_fn {
        f(ptr as *mut c_char);
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
