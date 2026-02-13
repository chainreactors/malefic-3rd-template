use malefic_3rd_ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    /// Nim runtime initializer — must be called once before any Nim code.
    fn NimMain();

    /// Returns the module name as a C string (static, do NOT free).
    fn NimModuleName() -> *const c_char;

    /// Synchronous handler: receives serialized Request, returns serialized Response.
    /// The returned buffer is malloc'd by Nim (via c_malloc); Rust frees it via NimFreeBuffer.
    fn NimModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;

    /// Frees a buffer allocated by the Nim side.
    fn NimFreeBuffer(ptr: *mut c_char);
}

static NIM_INIT: std::sync::Once = std::sync::Once::new();

pub struct NimModule {
    name: String,
}

impl NimModule {
    fn init() -> Self {
        NIM_INIT.call_once(|| unsafe { NimMain() });
        let name = unsafe { ffi_module_name(NimModuleName, None) };
        Self { name }
    }
}

#[async_trait]
impl Module for NimModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "nim_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        NimModule::init()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(NimModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for NimModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        _sender: &mut Output,
    ) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;

        // Encode Request to protobuf bytes
        let req_buf = encode_request(&request)?;

        // Call Nim handler (synchronous, blocking)
        let mut resp_ptr: *mut c_char = std::ptr::null_mut();
        let mut resp_len: c_int = 0;

        let rc = unsafe {
            NimModuleHandle(
                id as c_uint,
                req_buf.as_ptr() as *const c_char,
                req_buf.len() as c_int,
                &mut resp_ptr,
                &mut resp_len,
            )
        };

        if rc != 0 {
            return Err(anyhow!("NimModuleHandle failed (task {}, rc={})", id, rc).into());
        }

        // Decode Response from returned bytes using FfiBuffer for safe cleanup
        let response = if !resp_ptr.is_null() && resp_len > 0 {
            let buf = unsafe { FfiBuffer::new(resp_ptr, resp_len as usize, NimFreeBuffer) };
            decode_response(buf.as_bytes())?
        } else {
            if !resp_ptr.is_null() {
                unsafe { NimFreeBuffer(resp_ptr) };
            }
            Response::default()
        };

        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}

/// Register the Nim module into the bundle.
pub fn register(map: &mut MaleficBundle) {
    let module = NimModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
