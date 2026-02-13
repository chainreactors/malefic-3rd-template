use malefic_3rd_ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    /// Returns the module name as a C string (static, do NOT free).
    fn ZigModuleName() -> *const c_char;

    /// Synchronous handler: receives serialized Request, returns serialized Response.
    /// The returned buffer is malloc'd by Zig (via std.c.malloc); Rust frees it via ZigFreeBuffer.
    fn ZigModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;

    /// Frees a buffer allocated by the Zig side.
    fn ZigFreeBuffer(ptr: *mut c_char);
}

pub struct ZigModule {
    name: String,
}

impl ZigModule {
    fn init() -> Self {
        let name = unsafe { ffi_module_name(ZigModuleName, None) };
        Self { name }
    }
}

#[async_trait]
impl Module for ZigModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "zig_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        ZigModule::init()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(ZigModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for ZigModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        _sender: &mut Output,
    ) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;

        // Encode Request to protobuf bytes
        let req_buf = encode_request(&request)?;

        // Call Zig handler (synchronous, blocking)
        let mut resp_ptr: *mut c_char = std::ptr::null_mut();
        let mut resp_len: c_int = 0;

        let rc = unsafe {
            ZigModuleHandle(
                id as c_uint,
                req_buf.as_ptr() as *const c_char,
                req_buf.len() as c_int,
                &mut resp_ptr,
                &mut resp_len,
            )
        };

        if rc != 0 {
            return Err(anyhow!("ZigModuleHandle failed (task {}, rc={})", id, rc).into());
        }

        // Decode Response from returned bytes using FfiBuffer for safe cleanup
        let response = if !resp_ptr.is_null() && resp_len > 0 {
            let buf = unsafe { FfiBuffer::new(resp_ptr, resp_len as usize, ZigFreeBuffer) };
            decode_response(buf.as_bytes())?
        } else {
            if !resp_ptr.is_null() {
                unsafe { ZigFreeBuffer(resp_ptr) };
            }
            Response::default()
        };

        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}

/// Register the Zig module into the bundle.
pub fn register(map: &mut MaleficBundle) {
    let module = ZigModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
