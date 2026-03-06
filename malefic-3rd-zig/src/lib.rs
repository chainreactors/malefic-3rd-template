use malefic_module::prelude::*;
use malefic_module::ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn ZigModuleName() -> *const c_char;
    fn ZigModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;
}

pub struct ZigModule {
    name: String,
}

impl ZigModule {
    fn init() -> Self {
        let name = unsafe { ffi_module_name(ZigModuleName, false) };
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
        sender: &mut Output,
    ) -> ModuleResult {
        ffi_run_loop(id, receiver, sender, ZigModuleHandle, "ZigModuleHandle").await
    }
}

pub fn register(map: &mut MaleficBundle) {
    let module = ZigModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
